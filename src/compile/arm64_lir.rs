// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! ARM64 LIR backend — lowers `&[LIROp]` to ARM64 machine code bytes.
//!
//! This backend implements standard Rust integer semantics (wrapping u64),
//! not Goldilocks field arithmetic. It is the native codegen path used by
//! `rustc_codegen_trident`.
//!
//! # Register allocation
//!
//! Virtual registers Reg(n) map to physical ARM64 registers:
//!
//!   Reg(0..7)   → x0..x7   (ABI argument/return registers)
//!   Reg(8..14)  → x9..x15  (caller-saved scratch)
//!   Reg(15..23) → x19..x27 (callee-saved)
//!
//! If a function uses callee-saved registers (Reg(15+)), the prologue saves
//! them and the epilogue restores them. Reg(24+) spills to stack.
//!
//! # Label resolution
//!
//! Two-pass: first pass emits placeholders and records label offsets; second
//! pass patches forward-branch placeholders.
//!
//! # Relocations
//!
//! `Call(name)` emits BL with zero offset and records a relocation. The
//! `relocations()` method exposes them for the Mach-O/ELF emitter.
//! `LoadAddr { symbol }` emits ADRP+ADD placeholders and records a GOT-page relocation.

use std::collections::HashMap;
use std::cell::RefCell;

use crate::ir::lir::{LIROp, Reg, Label};
use crate::ir::lir::lower::RegisterLowering;
use super::arm64_encoders as enc;

// ── Physical register mapping ─────────────────────────────────────

/// Map virtual register index to ARM64 physical register number.
/// Returns None if the index requires a stack spill (≥ 24).
fn phys(vr: Reg) -> Option<u8> {
    match vr.0 {
        0..=7 => Some(vr.0 as u8),           // x0-x7  (ABI)
        8..=14 => Some(vr.0 as u8 + 1),      // x9-x15 (caller-saved; skip x8)
        15..=23 => Some(vr.0 as u8 + 4),     // x19-x27 (callee-saved; skip x16/17/18)
        _ => None,                             // spill
    }
}

/// Resolve virtual register, panicking on unallocated spill (not yet implemented).
fn r(vr: Reg) -> u8 {
    phys(vr).unwrap_or_else(|| panic!("Reg({}) requires spill — not yet implemented", vr.0))
}

// First callee-saved physical register (used to detect whether we need to save them).
const CALLEE_SAVED_FIRST: u8 = 19;
const CALLEE_SAVED_LAST: u8 = 27;
const FP: u8 = 29;  // frame pointer
const LR: u8 = 30;  // link register
const SP: u8 = 31;  // stack pointer

// ── Relocation descriptor ─────────────────────────────────────────

/// A relocation that the linker must resolve.
#[derive(Debug, Clone)]
pub struct Reloc {
    /// Byte offset within the emitted code where the patch is applied.
    pub offset: usize,
    /// Target symbol name (e.g. "_write", "_exit").
    pub symbol: String,
    pub kind: RelocKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelocKind {
    /// BL instruction — ARM64_RELOC_BRANCH26. 26-bit signed, in instructions.
    Branch26,
    /// ADRP instruction — ARM64_RELOC_PAGE21.
    Page21,
    /// ADD instruction after ADRP — ARM64_RELOC_PAGEOFF12.
    PageOff12,
}

// ── Backend ───────────────────────────────────────────────────────

/// ARM64 LIR lowering backend. Stateful due to relocation and label tracking.
pub struct Arm64LirBackend {
    relocations: RefCell<Vec<Reloc>>,
}

impl Arm64LirBackend {
    pub fn new() -> Self {
        Self { relocations: RefCell::new(Vec::new()) }
    }

    /// Relocations collected during the last `lower()` call.
    pub fn relocations(&self) -> Vec<Reloc> {
        self.relocations.borrow().clone()
    }
}

impl Default for Arm64LirBackend {
    fn default() -> Self { Self::new() }
}

impl RegisterLowering for Arm64LirBackend {
    fn target_name(&self) -> &str { "arm64" }

    fn lower(&self, ops: &[LIROp]) -> Vec<u8> {
        let mut ctx = LowerCtx::new();
        ctx.lower_ops(ops);
        *self.relocations.borrow_mut() = ctx.relocs;
        ctx.code
    }
}

// ── Lowering context ──────────────────────────────────────────────

struct LowerCtx {
    code: Vec<u8>,
    /// label name → byte offset in `code`
    labels: HashMap<String, usize>,
    /// (patch_offset_in_bytes, label_name, patch_kind)
    patches: Vec<(usize, String, PatchKind)>,
    relocs: Vec<Reloc>,
    /// Callee-saved regs used in this function (need save/restore).
    callee_used: Vec<u8>,
    /// Byte offset of the function prologue's STP x29,x30 instruction,
    /// for patching the frame size.
    prologue_stp_offset: Option<usize>,
    /// Number of 16-byte stack slots allocated (for callee-saved pairs).
    stack_slots: i8,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PatchKind {
    /// B or BL — patch imm26 field (bits 25:0)
    B26,
    /// CBNZ/CBZ — patch imm19 field (bits 23:5)
    B19,
}

impl LowerCtx {
    fn new() -> Self {
        Self {
            code: Vec::with_capacity(512),
            labels: HashMap::new(),
            patches: Vec::new(),
            relocs: Vec::new(),
            callee_used: Vec::new(),
            prologue_stp_offset: None,
            stack_slots: 0,
        }
    }

    fn emit(&mut self, insn: u32) {
        self.code.extend_from_slice(&insn.to_le_bytes());
    }

    fn emit_many(&mut self, insns: Vec<u32>) {
        for i in insns { self.emit(i); }
    }

    fn offset(&self) -> usize { self.code.len() }

    // ── Label handling ────────────────────────────────────────────

    fn def_label(&mut self, name: &str) {
        self.labels.insert(name.to_string(), self.offset());
    }

    fn emit_b_placeholder(&mut self, target: &str) {
        let off = self.offset();
        self.patches.push((off, target.to_string(), PatchKind::B26));
        self.emit(0x14000000); // B #0 placeholder
    }

    fn emit_cbnz_placeholder(&mut self, rn: u8, target: &str) {
        let off = self.offset();
        self.patches.push((off, target.to_string(), PatchKind::B19));
        self.emit(0xB5000000 | (rn as u32)); // CBNZ rn, #0 placeholder
    }

    fn emit_cbz_placeholder(&mut self, rn: u8, target: &str) {
        let off = self.offset();
        self.patches.push((off, target.to_string(), PatchKind::B19));
        self.emit(0xB4000000 | (rn as u32)); // CBZ rn, #0 placeholder
    }

    fn patch_branches(&mut self) {
        for (patch_off, label, kind) in &self.patches {
            let target_off = match self.labels.get(label) {
                Some(&o) => o,
                None => panic!("undefined label: {}", label),
            };
            let delta_bytes = target_off as i64 - *patch_off as i64;
            let delta_insns = delta_bytes / 4;
            let word = u32::from_le_bytes(
                self.code[*patch_off..*patch_off + 4].try_into().unwrap()
            );
            let patched = match kind {
                PatchKind::B26 => {
                    let imm26 = (delta_insns as u32) & 0x03FF_FFFF;
                    (word & 0xFC00_0000) | imm26
                }
                PatchKind::B19 => {
                    let imm19 = (delta_insns as u32) & 0x7FFFF;
                    (word & !0x00FFFFE0) | (imm19 << 5)
                }
            };
            self.code[*patch_off..*patch_off + 4].copy_from_slice(&patched.to_le_bytes());
        }
        self.patches.clear();
    }

    // ── Callee-saved scan ─────────────────────────────────────────

    fn scan_callee_used(ops: &[LIROp]) -> Vec<u8> {
        let mut used = std::collections::BTreeSet::new();
        for op in ops {
            let regs: Vec<Reg> = match op {
                LIROp::FnStart(_) | LIROp::FnEnd => continue,
                LIROp::LoadImm(d, _) | LIROp::Neg(d, _) | LIROp::Not(d, _)
                | LIROp::Invert(d, _) | LIROp::Log2(d, _) | LIROp::PopCount(d, _)
                | LIROp::SpongeInit(d) | LIROp::ExtInvert(d, _) => vec![*d],
                LIROp::Move(d, s) | LIROp::Eq(d, s, _) | LIROp::Lt(d, s, _) => vec![*d, *s],
                LIROp::Add(d, a, b) | LIROp::Sub(d, a, b) | LIROp::Mul(d, a, b)
                | LIROp::And(d, a, b) | LIROp::Or(d, a, b) | LIROp::Xor(d, a, b)
                | LIROp::Shl(d, a, b) | LIROp::Shr(d, a, b) | LIROp::Pow(d, a, b)
                | LIROp::ExtMul(d, a, b) => vec![*d, *a, *b],
                LIROp::LoadAddr { dst, .. } => vec![*dst],
                LIROp::Load { dst, base, .. } => vec![*dst, *base],
                LIROp::Store { src, base, .. } => vec![*src, *base],
                LIROp::DivMod { dst_quot, dst_rem, src1, src2 } =>
                    vec![*dst_quot, *dst_rem, *src1, *src2],
                _ => vec![],
            };
            for reg in regs {
                if let Some(p) = phys(reg) {
                    if (CALLEE_SAVED_FIRST..=CALLEE_SAVED_LAST).contains(&p) {
                        used.insert(p);
                    }
                }
            }
        }
        used.into_iter().collect()
    }

    // ── Prologue / epilogue ───────────────────────────────────────

    fn emit_prologue(&mut self, callee_used: Vec<u8>) {
        // Save FP + LR
        let off = self.offset();
        self.emit(enc::stp_pre(FP, LR, SP, -2)); // STP x29,x30,[sp,#-16]!
        self.prologue_stp_offset = Some(off);
        self.emit(enc::add_imm(FP, SP, 0));       // MOV x29, sp

        // Save callee-saved regs in pairs (collect before assigning to self)
        let pairs: Vec<Vec<u8>> = callee_used.chunks(2)
            .map(|c| c.to_vec()).collect();
        self.stack_slots = pairs.len() as i8;
        for chunk in &pairs {
            if chunk.len() == 2 {
                self.emit(enc::stp_pre(chunk[0], chunk[1], SP, -2));
            } else {
                self.emit(enc::stp_pre(chunk[0], chunk[0], SP, -2));
            }
        }
        self.callee_used = callee_used;
    }

    fn emit_epilogue(&mut self) {
        // Restore callee-saved regs in reverse order
        let pairs: Vec<Vec<u8>> = self.callee_used.chunks(2)
            .map(|c| c.to_vec()).collect::<Vec<_>>();
        for chunk in pairs.into_iter().rev() {
            if chunk.len() == 2 {
                self.emit(enc::ldp_post(chunk[0], chunk[1], SP, 2));
            } else {
                self.emit(enc::ldp_post(chunk[0], chunk[0], SP, 2));
            }
        }
        // Restore FP + LR, return
        self.emit(enc::ldp_post(FP, LR, SP, 2));
        self.emit(enc::ret());
    }

    // ── Main lowering loop ────────────────────────────────────────

    fn lower_ops(&mut self, ops: &[LIROp]) {
        // Pre-scan for callee-saved regs used in this function
        let fn_ops: &[LIROp] = ops;
        let callee_used = Self::scan_callee_used(fn_ops);

        for op in ops {
            match op {
                LIROp::Comment(_) => {}

                LIROp::FnStart(_name) => {
                    self.emit_prologue(callee_used.clone());
                }

                LIROp::FnEnd => {
                    self.emit_epilogue();
                    self.patch_branches();
                }

                LIROp::Entry(_) => {}

                LIROp::LabelDef(Label(name)) => {
                    self.def_label(name);
                }

                // ── Control flow ──────────────────────────────────

                LIROp::Return => {
                    self.emit_epilogue();
                }

                LIROp::Halt => {
                    // exit(0): x0=0, x16=1, svc #0x80
                    self.emit_many(enc::mov_imm64(0, 0));
                    self.emit_many(enc::mov_imm64(16, 1));
                    self.emit(enc::svc(0x80));
                }

                LIROp::Jump(Label(name)) => {
                    self.emit_b_placeholder(name);
                }

                LIROp::Branch { cond, if_true, if_false } => {
                    let rn = r(*cond);
                    self.emit_cbnz_placeholder(rn, &if_true.0);
                    self.emit_b_placeholder(&if_false.0);
                }

                LIROp::Call(name) => {
                    let off = self.offset();
                    self.emit(enc::bl_placeholder());
                    self.relocs.push(Reloc {
                        offset: off,
                        symbol: name.clone(),
                        kind: RelocKind::Branch26,
                    });
                }

                // ── Register ops ──────────────────────────────────

                LIROp::LoadImm(dst, val) => {
                    self.emit_many(enc::mov_imm64(r(*dst), *val));
                }

                LIROp::Move(dst, src) => {
                    let (d, s) = (r(*dst), r(*src));
                    if d != s { self.emit(enc::mov_reg(d, s)); }
                }

                LIROp::LoadAddr { dst, symbol } => {
                    let d = r(*dst);
                    let adrp_off = self.offset();
                    self.emit(enc::adrp(d, 0)); // placeholder
                    self.relocs.push(Reloc {
                        offset: adrp_off,
                        symbol: symbol.clone(),
                        kind: RelocKind::Page21,
                    });
                    let add_off = self.offset();
                    self.emit(enc::add_imm(d, d, 0)); // placeholder
                    self.relocs.push(Reloc {
                        offset: add_off,
                        symbol: symbol.clone(),
                        kind: RelocKind::PageOff12,
                    });
                }

                // ── Arithmetic ────────────────────────────────────

                LIROp::Add(d, a, b) => {
                    self.emit(enc::add(r(*d), r(*a), r(*b)));
                }

                LIROp::Sub(d, a, b) => {
                    self.emit(enc::sub(r(*d), r(*a), r(*b)));
                }

                LIROp::Mul(d, a, b) => {
                    self.emit(enc::mul(r(*d), r(*a), r(*b)));
                }

                LIROp::Neg(d, s) => {
                    self.emit(enc::neg(r(*d), r(*s)));
                }

                LIROp::Not(d, s) => {
                    self.emit(enc::mvn(r(*d), r(*s)));
                }

                LIROp::DivMod { dst_quot, dst_rem, src1, src2 } => {
                    let (q, rem, n, d) = (r(*dst_quot), r(*dst_rem), r(*src1), r(*src2));
                    self.emit(enc::udiv(q, n, d));
                    self.emit(enc::msub(rem, q, d, n)); // rem = n - q*d
                }

                LIROp::Shl(d, a, b) => {
                    self.emit(enc::lsl_reg(r(*d), r(*a), r(*b)));
                }

                LIROp::Shr(d, a, b) => {
                    self.emit(enc::lsr_reg(r(*d), r(*a), r(*b)));
                }

                LIROp::And(d, a, b) => {
                    self.emit(enc::and(r(*d), r(*a), r(*b)));
                }

                LIROp::Or(d, a, b) => {
                    self.emit(enc::orr(r(*d), r(*a), r(*b)));
                }

                LIROp::Xor(d, a, b) => {
                    self.emit(enc::eor(r(*d), r(*a), r(*b)));
                }

                LIROp::Eq(d, a, b) => {
                    self.emit(enc::cmp(r(*a), r(*b)));
                    self.emit(enc::cset(r(*d), 0)); // cset, EQ=0
                }

                LIROp::Lt(d, a, b) => {
                    self.emit(enc::cmp(r(*a), r(*b)));
                    self.emit(enc::cset(r(*d), 3)); // cset, CC/LO=3 (unsigned <)
                }

                LIROp::Log2(d, s) => {
                    // floor(log2(x)) = 63 - CLZ(x)
                    // Use a scratch immediate. We need a temp reg — use x16.
                    self.emit(enc::clz(16, r(*s)));
                    self.emit_many(enc::mov_imm64(17, 63));
                    self.emit(enc::sub(r(*d), 17, 16));
                }

                LIROp::PopCount(d, s) => {
                    // Software popcount: parallel bit-counting
                    // Uses x16 as temp. 64-bit Hamming weight.
                    let rs = r(*s);
                    let rd = r(*d);
                    // x16 = s
                    self.emit(enc::mov_reg(16, rs));
                    // bit-parallel popcount: 6 steps
                    // Step 1: x = x - ((x >> 1) & 0x5555...)
                    self.emit(enc::lsr_imm(17, 16, 1));
                    self.emit_many(enc::mov_imm64(rd, 0x5555_5555_5555_5555));
                    self.emit(enc::and(17, 17, rd));
                    self.emit(enc::sub(16, 16, 17));
                    // Step 2: x = (x & 0x3333...) + ((x >> 2) & 0x3333...)
                    self.emit_many(enc::mov_imm64(rd, 0x3333_3333_3333_3333));
                    self.emit(enc::and(17, 16, rd));
                    self.emit(enc::lsr_imm(16, 16, 2));
                    self.emit(enc::and(16, 16, rd));
                    self.emit(enc::add(16, 17, 16));
                    // Step 3: x = (x + (x >> 4)) & 0x0f0f...
                    self.emit(enc::lsr_imm(17, 16, 4));
                    self.emit(enc::add(16, 16, 17));
                    self.emit_many(enc::mov_imm64(rd, 0x0F0F_0F0F_0F0F_0F0F));
                    self.emit(enc::and(16, 16, rd));
                    // Step 4: x = (x * 0x0101...) >> 56
                    self.emit_many(enc::mov_imm64(rd, 0x0101_0101_0101_0101));
                    self.emit(enc::mul(16, 16, rd));
                    self.emit(enc::lsr_imm(rd, 16, 56));
                }

                LIROp::Pow(d, base, exp) => {
                    // Software exponentiation by squaring. Uses x16/x17 as temps.
                    let (rd, rb, re) = (r(*d), r(*base), r(*exp));
                    // result = 1
                    self.emit_many(enc::mov_imm64(rd, 1));
                    // base_reg = base (copy to x16 to avoid aliasing)
                    self.emit(enc::mov_reg(16, rb));
                    // exp_reg = exp (copy to x17)
                    self.emit(enc::mov_reg(17, re));
                    // loop: while exp != 0
                    let loop_start = Label::new(format!("__pow_loop_{}", self.offset()));
                    let loop_end = Label::new(format!("__pow_end_{}", self.offset()));
                    self.def_label(&loop_start.0);
                    self.emit_cbz_placeholder(17, &loop_end.0); // if exp==0 break
                    // if (exp & 1): result *= base
                    self.emit_many(enc::mov_imm64(15, 1));
                    self.emit(enc::and(15, 17, 15));  // x15 = exp & 1
                    let skip = Label::new(format!("__pow_skip_{}", self.offset()));
                    self.emit_cbz_placeholder(15, &skip.0);
                    self.emit(enc::mul(rd, rd, 16)); // result *= base
                    self.def_label(&skip.0);
                    self.emit(enc::mul(16, 16, 16)); // base *= base
                    self.emit(enc::lsr_imm(17, 17, 1)); // exp >>= 1
                    self.emit_b_placeholder(&loop_start.0);
                    self.def_label(&loop_end.0);
                }

                // Invert is field-specific — unsupported in standard Rust mode
                LIROp::Invert(_, _) => {
                    self.emit(enc::brk(0xFFFF)); // trap — not supported in native mode
                }

                LIROp::Split { dst_hi, dst_lo, src } => {
                    let (dh, dl, s) = (r(*dst_hi), r(*dst_lo), r(*src));
                    self.emit(enc::lsr_imm(dh, s, 32));
                    self.emit_many(enc::mov_imm64(16, 0xFFFF_FFFF));
                    self.emit(enc::and(dl, s, 16));
                }

                // ── Memory ────────────────────────────────────────

                LIROp::Load { dst, base, offset } => {
                    let (d, b) = (r(*dst), r(*base));
                    if *offset >= 0 && (*offset % 8 == 0) && (*offset / 8 < 4096) {
                        self.emit(enc::ldr_imm(d, b, (*offset / 8) as u16));
                    } else if (-256..=255).contains(offset) {
                        self.emit(enc::ldur(d, b, *offset as i16));
                    } else {
                        // Large offset: load offset into x16 and use register form
                        self.emit_many(enc::mov_imm64(16, *offset as u64));
                        self.emit(enc::add(16, b, 16));
                        self.emit(enc::ldr_imm(d, 16, 0));
                    }
                }

                LIROp::Store { src, base, offset } => {
                    let (s, b) = (r(*src), r(*base));
                    if *offset >= 0 && (*offset % 8 == 0) && (*offset / 8 < 4096) {
                        self.emit(enc::str_imm(s, b, (*offset / 8) as u16));
                    } else if (-256..=255).contains(offset) {
                        self.emit(enc::stur(s, b, *offset as i16));
                    } else {
                        self.emit_many(enc::mov_imm64(16, *offset as u64));
                        self.emit(enc::add(16, b, 16));
                        self.emit(enc::str_imm(s, 16, 0));
                    }
                }

                LIROp::LoadMulti { dst, base, width } => {
                    for i in 0..*width {
                        let vd = Reg(dst.0 + i);
                        let d = r(vd);
                        let b = r(*base);
                        let off_units = i as u16;
                        self.emit(enc::ldr_imm(d, b, off_units));
                    }
                }

                LIROp::StoreMulti { src, base, width } => {
                    for i in 0..*width {
                        let vs = Reg(src.0 + i);
                        let s = r(vs);
                        let b = r(*base);
                        let off_units = i as u16;
                        self.emit(enc::str_imm(s, b, off_units));
                    }
                }

                // ── I/O, Hash, Events, RAM — not implemented for native mode ──
                // These are proof VM ops. Emit a trap.
                LIROp::ReadIo { .. } | LIROp::WriteIo { .. } | LIROp::Hint { .. }
                | LIROp::Hash { .. } | LIROp::Assert { .. }
                | LIROp::Reveal { .. } | LIROp::Seal { .. }
                | LIROp::RamRead { .. } | LIROp::RamWrite { .. } => {
                    self.emit(enc::brk(0x0001)); // proof-only op in native context
                }

                // ── Tier 2/3 — proof VM only ──────────────────────
                LIROp::SpongeInit(_) | LIROp::SpongeAbsorb { .. }
                | LIROp::SpongeSqueeze { .. } | LIROp::SpongeLoad { .. }
                | LIROp::MerkleStep { .. } | LIROp::MerkleLoad { .. }
                | LIROp::ExtMul(..) | LIROp::ExtInvert(..)
                | LIROp::FoldExt { .. } | LIROp::FoldBase { .. }
                | LIROp::ProofBlock { .. } | LIROp::ProofBlockEnd => {
                    self.emit(enc::brk(0x0002)); // proof-only
                }

                // ── Inline asm passthrough ────────────────────────
                LIROp::Asm { lines } => {
                    for line in lines {
                        match assemble_line(line.trim()) {
                            Some(insn) => self.emit(insn),
                            None => self.emit(enc::brk(0xFFFF)), // unrecognized asm
                        }
                    }
                }
            }
        }
    }
}

// ── Micro-assembler for inline asm passthrough ────────────────────

/// Encode a single ARM64 assembly line (trimmed, lowercase accepted).
/// Only covers the instructions needed for syscall stubs and common inline asm.
fn assemble_line(line: &str) -> Option<u32> {
    let line = line.to_lowercase();
    let line = line.trim();

    if line == "ret" { return Some(enc::ret()); }
    if line == "nop" { return Some(enc::nop()); }

    // svc #imm
    if let Some(rest) = line.strip_prefix("svc #") {
        let imm: u16 = rest.trim().parse().ok()?;
        return Some(enc::svc(imm));
    }
    if let Some(rest) = line.strip_prefix("svc 0x") {
        let imm = u16::from_str_radix(rest.trim(), 16).ok()?;
        return Some(enc::svc(imm));
    }

    // brk #imm
    if let Some(rest) = line.strip_prefix("brk #") {
        let imm: u16 = rest.trim().parse().ok()?;
        return Some(enc::brk(imm));
    }

    None
}

// ── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::lir::LIROp;

    fn lower(ops: &[LIROp]) -> Vec<u8> {
        let backend = Arm64LirBackend::new();
        backend.lower(ops)
    }

    #[test]
    fn test_load_imm_small() {
        let ops = [LIROp::FnStart("f".into()),
                   LIROp::LoadImm(Reg(0), 42),
                   LIROp::FnEnd];
        let code = lower(&ops);
        assert!(!code.is_empty());
        // MOVZ x0, #42  = 0xD2800540
        let movz = u32::from_le_bytes(code[8..12].try_into().unwrap()); // after prologue
        assert_eq!(movz & 0xFFE0001F, 0xD2800000); // opcode mask
        assert_eq!((movz >> 5) & 0xFFFF, 42);       // immediate
    }

    #[test]
    fn test_add() {
        let ops = [LIROp::FnStart("f".into()),
                   LIROp::Add(Reg(8), Reg(0), Reg(1)),
                   LIROp::FnEnd];
        let code = lower(&ops);
        // Find ADD x9, x0, x1 (Reg(8) → x9, Reg(0) → x0, Reg(1) → x1)
        let mut found = false;
        for i in (0..code.len()).step_by(4) {
            let w = u32::from_le_bytes(code[i..i+4].try_into().unwrap());
            if w == enc::add(9, 0, 1) { found = true; break; }
        }
        assert!(found, "ADD x9, x0, x1 not found in output");
    }

    #[test]
    fn test_hello_world_sequence() {
        // Minimal LIR for: write(1, msg, 14); exit(0)
        let ops = vec![
            LIROp::FnStart("_main".into()),
            LIROp::LoadImm(Reg(0), 1),
            LIROp::LoadAddr { dst: Reg(1), symbol: "__msg".into() },
            LIROp::LoadImm(Reg(2), 14),
            LIROp::Call("_write".into()),
            LIROp::LoadImm(Reg(0), 0),
            LIROp::Call("_exit".into()),
            LIROp::Halt,
            LIROp::FnEnd,
        ];
        let backend = Arm64LirBackend::new();
        let code = backend.lower(&ops);
        let relocs = backend.relocations();
        assert!(!code.is_empty());
        // Must have relocations for _write and _exit (2 BL + 2 ADRP/ADD for __msg)
        let call_relocs: Vec<_> = relocs.iter()
            .filter(|r| r.kind == RelocKind::Branch26).collect();
        assert_eq!(call_relocs.len(), 2);
        assert_eq!(call_relocs[0].symbol, "_write");
        assert_eq!(call_relocs[1].symbol, "_exit");
    }

    #[test]
    fn test_branch_patch() {
        let ops = vec![
            LIROp::FnStart("f".into()),
            LIROp::LoadImm(Reg(0), 1),
            LIROp::Branch {
                cond: Reg(0),
                if_true: Label::new("yes"),
                if_false: Label::new("no"),
            },
            LIROp::LabelDef(Label::new("yes")),
            LIROp::LoadImm(Reg(0), 1),
            LIROp::Jump(Label::new("end")),
            LIROp::LabelDef(Label::new("no")),
            LIROp::LoadImm(Reg(0), 0),
            LIROp::LabelDef(Label::new("end")),
            LIROp::Return,
            LIROp::FnEnd,
        ];
        let code = lower(&ops);
        // Just ensure it doesn't panic and produces non-empty output
        assert!(code.len() > 16);
    }
}
