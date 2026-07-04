//! ARM64 JIT compiler for nox formulas
//!
//! Compiles a nox formula into ARM64 machine code, maps it into
//! executable memory, and runs it as a native function.
//!
//! Phase 1: atom-only — subject params passed via x0-x7 (up to 8),
//! result returned in x0. All values are Goldilocks field elements (u64).
//!
//! Register allocation:
//!   x0-x7:  function parameters / scratch
//!   x9-x15: scratch registers for intermediate values
//!   x16-x17: intra-procedure-call scratch
//!   x19-x28: callee-saved (we save/restore if used)
//!   x29: frame pointer, x30: link register
//!
//! We use a simple stack-based register allocator: each sub-formula
//! pushes its result onto a virtual register stack. When we run out
//! of physical registers, we spill to the real stack.




use nox::{Reduction as Order, Order as NounId};
use super::{CompileError, formula_parts, body_pair, body_triple, atom_u64, axis_to_param,
            detect_loop_setup, detect_back_edge};

const P: u64 = 0xFFFF_FFFF_0000_0001;
const MAX_PARAMS: u32 = 8; // ARM64 ABI: x0-x7

/// Compile a nox formula to ARM64 machine code.
pub fn compile_to_arm64<const N: usize>(
    order: &Order<N>,
    formula: NounId,
    num_params: u32,
) -> Result<Vec<u8>, CompileError> {
    if num_params > MAX_PARAMS {
        return Err(CompileError::NoParams);
    }
    let mut emitter = Arm64Emitter::new(num_params);
    emitter.emit_formula(order, formula)?;
    // Move result from top of virtual stack to x0
    let result_reg = emitter.pop_reg();
    if result_reg != 0 {
        emitter.emit_mov_reg(0, result_reg);
    }
    emitter.emit_ret();
    Ok(emitter.code)
}

#[derive(Clone)]
struct Arm64LoopState {
    carried: Vec<u8>,         // registers holding carried locals
    formula_reg: u8,          // register for formula slot
    header_offset: usize,     // byte offset of loop header in code
}

struct Arm64Emitter {
    code: Vec<u8>,
    /// Virtual register stack. Each formula result goes here.
    reg_stack: Vec<u8>,
    /// Next available scratch register
    next_scratch: u8,
    /// Subject model: maps depth → register number.
    /// depth 0 = head (shallowest), depth N = deepest.
    subject: Vec<u8>,
    /// Current loop state (for back-edge emission)
    loop_state: Option<Arm64LoopState>,
}

// Scratch registers: x9, x10, x11, x12, x13, x14, x15
const SCRATCH_BASE: u8 = 9;
const SCRATCH_COUNT: u8 = 7;

impl Arm64Emitter {
    fn new(num_params: u32) -> Self {
        // Initial subject: params in reverse order (last param = depth 0)
        let subject: Vec<u8> = (0..num_params).rev().map(|i| i as u8).collect();
        Self {
            code: Vec::with_capacity(512),
            reg_stack: Vec::with_capacity(16),
            next_scratch: 0,
            subject,
            loop_state: None,
        }
    }

    /// Allocate a scratch register for a formula result.
    fn push_reg(&mut self) -> u8 {
        let reg = SCRATCH_BASE + (self.next_scratch % SCRATCH_COUNT);
        self.next_scratch += 1;
        self.reg_stack.push(reg);
        reg
    }

    /// Pop the top register from the virtual stack.
    fn pop_reg(&mut self) -> u8 {
        self.next_scratch -= 1;
        self.reg_stack.pop().unwrap_or(SCRATCH_BASE)
    }

    fn emit_u32(&mut self, insn: u32) {
        self.code.extend_from_slice(&insn.to_le_bytes());
    }

    fn emit_formula<const N: usize>(&mut self, order: &Order<N>, formula: NounId) -> Result<(), CompileError> {
        let (tag, body) = formula_parts(order, formula)?;
        match tag {
            0 => self.emit_axis(order, body),
            1 => self.emit_quote(order, body),
            4 => self.emit_branch(order, body),
            5 => self.emit_add(order, body),
            6 => self.emit_sub(order, body),
            7 => self.emit_mul(order, body),
            9 => self.emit_eq(order, body),
            10 => self.emit_lt(order, body),
            11 => self.emit_xor(order, body),
            12 => self.emit_and(order, body),
            13 => self.emit_not(order, body),
            2 => self.emit_compose(order, body),
            14 => self.emit_shl(order, body),
            _ => Err(CompileError::UnsupportedPattern(tag)),
        }
    }

    // ── pattern emitters ──────────────────────────────────────────

    fn emit_axis<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let addr = atom_u64(order, body)?;
        let depth = axis_to_param(addr)?;
        if (depth as usize) >= self.subject.len() { return Err(CompileError::NoParams); }
        let src_reg = self.subject[depth as usize];
        let dst = self.push_reg();
        self.emit_mov_reg(dst, src_reg);
        Ok(())
    }

    fn emit_compose<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        // Check for loop setup
        if let Some((loop_body, inits)) = detect_loop_setup(order, body) {
            return self.emit_loop(order, loop_body, &inits);
        }

        // Check for back-edge
        if let Some((new_subj, _axis)) = detect_back_edge(order, body) {
            return self.emit_back_edge(order, new_subj);
        }

        // Let-binding
        let (a_formula, b_formula) = body_pair(order, body)?;

        let (a_tag, a_body) = formula_parts(order, a_formula)?;
        if a_tag != 3 { return Err(CompileError::UnsupportedPattern(2)); }
        let (value_formula, identity) = body_pair(order, a_body)?;
        let (id_tag, id_body) = formula_parts(order, identity)?;
        if id_tag != 0 || atom_u64(order, id_body)? != 1 {
            return Err(CompileError::UnsupportedPattern(2));
        }
        let (b_tag, body_formula) = formula_parts(order, b_formula)?;
        if b_tag != 1 { return Err(CompileError::UnsupportedPattern(2)); }

        // Compile value, result in a register
        self.emit_formula(order, value_formula)?;
        let val_reg = self.pop_reg();

        // Push register onto subject model
        self.subject.insert(0, val_reg);

        let result = self.emit_formula(order, body_formula);

        // Restore subject
        self.subject.remove(0);

        result
    }

    fn emit_loop<const N: usize>(
        &mut self, order: &Order<N>, loop_body: NounId, inits: &[NounId],
    ) -> Result<(), CompileError> {
        // Allocate registers for formula slot + carried locals
        let formula_reg = SCRATCH_BASE + (self.next_scratch % SCRATCH_COUNT);
        self.next_scratch += 1;

        let mut carried_regs = Vec::new();
        for _ in 0..inits.len() {
            let r = SCRATCH_BASE + (self.next_scratch % SCRATCH_COUNT);
            self.next_scratch += 1;
            carried_regs.push(r);
        }

        // Compile init values and store into carried registers
        for (i, &init) in inits.iter().enumerate() {
            self.emit_formula(order, init)?;
            let val = self.pop_reg();
            if val != carried_regs[i] {
                self.emit_mov_reg(carried_regs[i], val);
            }
        }

        // Initialize formula_reg to 0 (placeholder)
        self.emit_mov_imm64(formula_reg, 0);

        // Build loop subject: push carried locals then formula slot
        let saved_subject = self.subject.clone();
        for &cl in carried_regs.iter() {
            self.subject.insert(0, cl);
        }
        self.subject.insert(0, formula_reg);

        // Save loop state
        let prev_loop = self.loop_state.take();
        let header_offset = self.code.len();
        self.loop_state = Some(Arm64LoopState {
            carried: carried_regs,
            formula_reg,
            header_offset,
        });

        // Compile loop body
        self.emit_formula(order, loop_body)?;

        // Restore
        self.loop_state = prev_loop;
        self.subject = saved_subject;
        Ok(())
    }

    fn emit_back_edge<const N: usize>(
        &mut self, order: &Order<N>, new_subj_formula: NounId,
    ) -> Result<(), CompileError> {
        let ls = self.loop_state.as_ref()
            .ok_or(CompileError::UnsupportedPattern(2))?
            .clone();

        // Walk cons chain: [3 [formula_ref [3 [c0 [3 [c1 ... params]]]]]]
        let (tag, cons_body) = formula_parts(order, new_subj_formula)?;
        if tag != 3 { return Err(CompileError::UnsupportedPattern(2)); }
        let (_formula_ref, rest) = body_pair(order, cons_body)?;

        let mut cur = rest;
        for &carried_reg in ls.carried.iter() {
            let (tag, cb) = formula_parts(order, cur)?;
            if tag != 3 { break; }
            let (val_formula, tail) = body_pair(order, cb)?;
            self.emit_formula(order, val_formula)?;
            let val = self.pop_reg();
            if val != carried_reg {
                self.emit_mov_reg(carried_reg, val);
            }
            cur = tail;
        }

        // Unconditional branch back to loop header
        let current = self.code.len();
        let offset = ls.header_offset as i32 - current as i32;
        let imm26 = ((offset / 4) as u32) & 0x03FF_FFFF;
        self.emit_u32(0x14000000 | imm26);

        // Push a dummy result on the reg stack (unreachable but keeps stack balanced)
        self.push_reg();
        Ok(())
    }

    fn emit_quote<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let val = atom_u64(order, body)?;
        let dst = self.push_reg();
        self.emit_mov_imm64(dst, val);
        Ok(())
    }

    fn emit_branch<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (test, yes, no) = body_triple(order, body)?;

        // Evaluate test
        self.emit_formula(order, test)?;
        let test_reg = self.pop_reg();

        // cbz test_reg, yes_label (nox: 0=yes)
        let cbz_offset = self.code.len();
        self.emit_u32(0); // placeholder CBZ

        // no branch (test != 0)
        self.emit_formula(order, no)?;
        let no_reg = self.pop_reg();
        let result = self.push_reg();
        if no_reg != result {
            self.emit_mov_reg(result, no_reg);
        }
        let b_offset = self.code.len();
        self.emit_u32(0); // placeholder B (skip yes)
        self.pop_reg(); // balance for the yes path below

        // yes label
        let yes_label = self.code.len();
        // Patch CBZ: cbz test_reg, yes_label
        let cbz_imm19 = ((yes_label - cbz_offset) / 4) as u32;
        let cbz = 0xB4000000 | (cbz_imm19 << 5) | (test_reg as u32);
        self.code[cbz_offset..cbz_offset + 4].copy_from_slice(&cbz.to_le_bytes());

        self.emit_formula(order, yes)?;
        let yes_reg = self.pop_reg();
        let result2 = self.push_reg();
        if yes_reg != result2 {
            self.emit_mov_reg(result2, yes_reg);
        }

        // end label
        let end_label = self.code.len();
        // Patch B
        let b_imm26 = ((end_label - b_offset) / 4) as u32;
        let b = 0x14000000 | b_imm26;
        self.code[b_offset..b_offset + 4].copy_from_slice(&b.to_le_bytes());

        Ok(())
    }

    fn emit_add<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();

        // adds dst, ra, rb (set flags for overflow detection)
        self.emit_u32(arm64_adds(dst, ra, rb));

        // Load P into x16
        self.emit_mov_imm64(16, P);

        // if carry (overflow): dst += 0xFFFFFFFF
        // b.cc no_overflow
        let bcc_off = self.code.len();
        self.emit_u32(0); // placeholder

        self.emit_mov_imm64(17, 0xFFFF_FFFF);
        self.emit_u32(arm64_add(dst, dst, 17));

        let no_overflow = self.code.len();
        let bcc_imm = ((no_overflow - bcc_off) / 4) as u32;
        // b.cc = b.lo (condition 0011 = CC/LO)
        let bcc = 0x54000003 | (bcc_imm << 5);
        self.code[bcc_off..bcc_off + 4].copy_from_slice(&bcc.to_le_bytes());

        // if dst >= P: dst -= P
        // cmp dst, x16
        self.emit_u32(arm64_cmp(dst, 16));
        // b.lo skip
        let blo_off = self.code.len();
        self.emit_u32(0);
        self.emit_u32(arm64_sub(dst, dst, 16));
        let skip = self.code.len();
        let blo_imm = ((skip - blo_off) / 4) as u32;
        let blo = 0x54000003 | (blo_imm << 5);
        self.code[blo_off..blo_off + 4].copy_from_slice(&blo.to_le_bytes());

        Ok(())
    }

    fn emit_sub<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();

        // cmp ra, rb
        self.emit_u32(arm64_cmp(ra, rb));

        // if ra >= rb: dst = ra - rb
        // b.lo underflow
        let blo_off = self.code.len();
        self.emit_u32(0);

        self.emit_u32(arm64_sub(dst, ra, rb));
        let b_off = self.code.len();
        self.emit_u32(0); // B end

        // underflow: dst = P - rb + ra
        let underflow = self.code.len();
        let blo_imm = ((underflow - blo_off) / 4) as u32;
        let blo = 0x54000003 | (blo_imm << 5);
        self.code[blo_off..blo_off + 4].copy_from_slice(&blo.to_le_bytes());

        self.emit_mov_imm64(16, P);
        self.emit_u32(arm64_sub(dst, 16, rb));
        self.emit_u32(arm64_add(dst, dst, ra));

        let end = self.code.len();
        let b_imm = ((end - b_off) / 4) as u32;
        let b = 0x14000000 | b_imm;
        self.code[b_off..b_off + 4].copy_from_slice(&b.to_le_bytes());

        Ok(())
    }

    fn emit_mul<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();

        // ARM64 has UMULH for upper 64 bits of 64×64 multiply!
        // lo = ra * rb
        self.emit_u32(arm64_mul(dst, ra, rb));      // MUL dst, ra, rb
        self.emit_u32(arm64_umulh(17, ra, rb));      // UMULH x17, ra, rb (hi)

        // Reduce: result = lo + hi * (2^32 - 1)  mod p
        // hi_shifted = hi << 32
        // result = lo + hi_shifted - hi
        // (with overflow/underflow handling)

        // x16 = hi << 32
        self.emit_u32(arm64_lsl(16, 17, 32));

        // adds dst, dst, x16 (lo + hi<<32, may overflow)
        self.emit_u32(arm64_adds(dst, dst, 16));

        // if carry: dst += 0xFFFFFFFF
        let bcc_off = self.code.len();
        self.emit_u32(0); // b.cc placeholder
        self.emit_mov_imm64(16, 0xFFFF_FFFF);
        self.emit_u32(arm64_add(dst, dst, 16));
        let no_carry = self.code.len();
        let bcc_imm = ((no_carry - bcc_off) / 4) as u32;
        let bcc = 0x54000003 | (bcc_imm << 5);
        self.code[bcc_off..bcc_off + 4].copy_from_slice(&bcc.to_le_bytes());

        // subs dst, dst, x17 (- hi, may underflow)
        self.emit_u32(arm64_subs(dst, dst, 17));

        // if borrow (carry clear after subs): dst -= 0xFFFFFFFF
        // b.cs no_borrow
        let bcs_off = self.code.len();
        self.emit_u32(0);
        self.emit_mov_imm64(16, 0xFFFF_FFFF);
        self.emit_u32(arm64_sub(dst, dst, 16));
        let no_borrow = self.code.len();
        let bcs_imm = ((no_borrow - bcs_off) / 4) as u32;
        // b.cs = condition 0010
        let bcs = 0x54000002 | (bcs_imm << 5);
        self.code[bcs_off..bcs_off + 4].copy_from_slice(&bcs.to_le_bytes());

        // Final reduce: if dst >= P, dst -= P
        self.emit_mov_imm64(16, P);
        self.emit_u32(arm64_cmp(dst, 16));
        let blo2_off = self.code.len();
        self.emit_u32(0);
        self.emit_u32(arm64_sub(dst, dst, 16));
        let end = self.code.len();
        let blo2_imm = ((end - blo2_off) / 4) as u32;
        let blo2 = 0x54000003 | (blo2_imm << 5);
        self.code[blo2_off..blo2_off + 4].copy_from_slice(&blo2.to_le_bytes());

        Ok(())
    }

    fn emit_eq<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();

        self.emit_u32(arm64_cmp(ra, rb));
        // cset dst, ne (nox: 0=equal, 1=not equal)
        // CSET Xd, ne = CSINC Xd, XZR, XZR, eq  (condition EQ=0)
        self.emit_u32(0x9A9F17E0 | (dst as u32)); // CSINC dst, xzr, xzr, eq → 1 if ne
        Ok(())
    }

    fn emit_lt<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();

        self.emit_u32(arm64_cmp(ra, rb));
        // nox: 0 if a<b, 1 if a>=b
        // cset dst, hs (hs = carry set = a >= b unsigned)
        // CSINC Xd, XZR, XZR, lo → 1 if hs
        self.emit_u32(0x9A9F37E0 | (dst as u32)); // CSINC dst, xzr, xzr, lo
        Ok(())
    }

    fn emit_xor<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();
        // EOR Xd, Xn, Xm
        self.emit_u32(0xCA000000 | ((rb as u32) << 16) | ((ra as u32) << 5) | (dst as u32));
        Ok(())
    }

    fn emit_and<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();
        // AND Xd, Xn, Xm
        self.emit_u32(0x8A000000 | ((rb as u32) << 16) | ((ra as u32) << 5) | (dst as u32));
        Ok(())
    }

    fn emit_not<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        self.emit_formula(order, body)?;
        let ra = self.pop_reg();
        let dst = self.push_reg();
        // MVN Xd, Xn  = ORN Xd, XZR, Xn
        self.emit_u32(0xAA2003E0 | ((ra as u32) << 16) | (dst as u32));
        // AND with 0xFFFFFFFF (mask to 32 bits)
        self.emit_mov_imm64(16, 0xFFFF_FFFF);
        self.emit_u32(0x8A000000 | (16u32 << 16) | ((dst as u32) << 5) | (dst as u32));
        Ok(())
    }

    fn emit_shl<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();
        // LSL Xd, Xn, Xm = LSLV Xd, Xn, Xm
        self.emit_u32(0x9AC02000 | ((rb as u32) << 16) | ((ra as u32) << 5) | (dst as u32));
        // Mask to 32 bits
        self.emit_mov_imm64(16, 0xFFFF_FFFF);
        self.emit_u32(0x8A000000 | (16u32 << 16) | ((dst as u32) << 5) | (dst as u32));
        Ok(())
    }

    // ── instruction helpers ───────────────────────────────────────

    fn emit_mov_reg(&mut self, dst: u8, src: u8) {
        if dst != src {
            // MOV Xd, Xm = ORR Xd, XZR, Xm
            self.emit_u32(0xAA0003E0 | ((src as u32) << 16) | (dst as u32));
        }
    }

    fn emit_mov_imm64(&mut self, reg: u8, val: u64) {
        // Use MOVZ + MOVK sequence for 64-bit immediates
        let r = reg as u32;
        // MOVZ Xd, imm16, lsl #0
        self.emit_u32(0xD2800000 | (((val & 0xFFFF) as u32) << 5) | r);
        if val > 0xFFFF {
            // MOVK Xd, imm16, lsl #16
            self.emit_u32(0xF2A00000 | ((((val >> 16) & 0xFFFF) as u32) << 5) | r);
        }
        if val > 0xFFFF_FFFF {
            // MOVK Xd, imm16, lsl #32
            self.emit_u32(0xF2C00000 | ((((val >> 32) & 0xFFFF) as u32) << 5) | r);
        }
        if val > 0xFFFF_FFFF_FFFF {
            // MOVK Xd, imm16, lsl #48
            self.emit_u32(0xF2E00000 | ((((val >> 48) & 0xFFFF) as u32) << 5) | r);
        }
    }

    fn emit_ret(&mut self) {
        self.emit_u32(0xD65F03C0); // RET
    }
}

// ── ARM64 instruction encoders ───────────────────────────────────

fn arm64_add(rd: u8, rn: u8, rm: u8) -> u32 {
    0x8B000000 | ((rm as u32) << 16) | ((rn as u32) << 5) | (rd as u32)
}

fn arm64_adds(rd: u8, rn: u8, rm: u8) -> u32 {
    0xAB000000 | ((rm as u32) << 16) | ((rn as u32) << 5) | (rd as u32)
}

fn arm64_sub(rd: u8, rn: u8, rm: u8) -> u32 {
    0xCB000000 | ((rm as u32) << 16) | ((rn as u32) << 5) | (rd as u32)
}

fn arm64_subs(rd: u8, rn: u8, rm: u8) -> u32 {
    0xEB000000 | ((rm as u32) << 16) | ((rn as u32) << 5) | (rd as u32)
}

fn arm64_mul(rd: u8, rn: u8, rm: u8) -> u32 {
    // MUL = MADD Xd, Xn, Xm, XZR
    0x9B007C00 | ((rm as u32) << 16) | ((rn as u32) << 5) | (rd as u32)
}

fn arm64_umulh(rd: u8, rn: u8, rm: u8) -> u32 {
    0x9BC07C00 | ((rm as u32) << 16) | ((rn as u32) << 5) | (rd as u32)
}

fn arm64_cmp(rn: u8, rm: u8) -> u32 {
    // CMP = SUBS XZR, Xn, Xm
    arm64_subs(31, rn, rm)
}

fn arm64_lsl(rd: u8, rn: u8, shift: u8) -> u32 {
    // LSL by immediate = UBFM Xd, Xn, #(-shift mod 64), #(63-shift)
    let immr = (64 - shift as u32) & 63;
    let imms = 63 - shift as u32;
    0xD3400000 | (immr << 16) | (imms << 10) | ((rn as u32) << 5) | (rd as u32)
}
