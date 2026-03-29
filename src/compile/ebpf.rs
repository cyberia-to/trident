//! eBPF code emitter for nox formulas
//!
//! Hand-emitted eBPF bytecode for Linux kernel execution.
//! Args in r1-r5 (max 5 params), return in r0.
//! Scratch: r6-r9 (callee-saved in BPF, but for standalone programs ok).
//! 64-bit fixed-size instructions (8 bytes each).
//! No MULHU — Goldilocks mul uses 32-bit schoolbook decomposition.




use nox::noun::{Order, NounId};
use super::{CompileError, formula_parts, body_pair, body_triple, atom_u64, axis_to_param,
            detect_loop_setup, detect_back_edge};

const P: u64 = 0xFFFF_FFFF_0000_0001;
const MAX_PARAMS: u32 = 5;

pub fn compile_to_ebpf<const N: usize>(
    order: &Order<N>,
    formula: NounId,
    num_params: u32,
) -> Result<Vec<u8>, CompileError> {
    if num_params > MAX_PARAMS { return Err(CompileError::NoParams); }
    let mut e = EbpfEmitter::new(num_params);
    e.emit_formula(order, formula)?;
    let result_reg = e.pop_reg();
    // Move result to r0
    if result_reg != 0 {
        e.emit(ebpf_mov64(0, result_reg));
    }
    e.emit(ebpf_exit());
    Ok(e.code)
}

#[derive(Clone)]
struct EbpfLoopState {
    carried: Vec<u8>,         // registers holding carried locals
    formula_reg: u8,          // register for formula slot
    header_idx: usize,        // instruction index of loop header
}

struct EbpfEmitter {
    code: Vec<u8>,
    reg_stack: Vec<u8>,
    next_scratch: u8,
    subject: Vec<u8>,
    loop_state: Option<EbpfLoopState>,
}

// Scratch registers: r6, r7, r8, r9
const SCRATCH_REGS: [u8; 4] = [6, 7, 8, 9];

impl EbpfEmitter {
    fn new(num_params: u32) -> Self {
        // Args: r1-r5. Subject: last param = depth 0 (head)
        let subject: Vec<u8> = (0..num_params).rev().map(|i| (1 + i) as u8).collect();
        Self { code: Vec::with_capacity(512), reg_stack: Vec::new(), next_scratch: 0, subject, loop_state: None }
    }

    fn push_reg(&mut self) -> u8 {
        let reg = SCRATCH_REGS[(self.next_scratch as usize) % SCRATCH_REGS.len()];
        self.next_scratch += 1;
        self.reg_stack.push(reg);
        reg
    }

    fn pop_reg(&mut self) -> u8 {
        self.next_scratch -= 1;
        self.reg_stack.pop().unwrap_or(SCRATCH_REGS[0])
    }

    fn emit(&mut self, insn: u64) {
        self.code.extend_from_slice(&insn.to_le_bytes());
    }

    /// Current instruction index (for branch offset calculation)
    fn insn_idx(&self) -> usize {
        self.code.len() / 8
    }

    fn emit_formula<const N: usize>(&mut self, order: &Order<N>, formula: NounId) -> Result<(), CompileError> {
        let (tag, body) = formula_parts(order, formula)?;
        match tag {
            0 => self.emit_axis(order, body),
            1 => self.emit_quote(order, body),
            2 => self.emit_compose(order, body),
            4 => self.emit_branch(order, body),
            5 => self.emit_add(order, body),
            6 => self.emit_sub(order, body),
            7 => self.emit_mul(order, body),
            9 => self.emit_eq(order, body),
            10 => self.emit_lt(order, body),
            11 => self.emit_xor(order, body),
            12 => self.emit_and(order, body),
            13 => self.emit_not(order, body),
            14 => self.emit_shl(order, body),
            _ => Err(CompileError::UnsupportedPattern(tag)),
        }
    }

    fn emit_axis<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let addr = atom_u64(order, body)?;
        let depth = axis_to_param(addr)?;
        if (depth as usize) >= self.subject.len() { return Err(CompileError::NoParams); }
        let src = self.subject[depth as usize];
        let dst = self.push_reg();
        self.emit(ebpf_mov64(dst, src));
        Ok(())
    }

    fn emit_quote<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let val = atom_u64(order, body)?;
        let dst = self.push_reg();
        self.emit_load_imm64(dst, val);
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

        self.emit_formula(order, value_formula)?;
        let val_reg = self.pop_reg();
        self.subject.insert(0, val_reg);
        let result = self.emit_formula(order, body_formula);
        self.subject.remove(0);
        result
    }

    fn emit_loop<const N: usize>(
        &mut self, order: &Order<N>, loop_body: NounId, inits: &[NounId],
    ) -> Result<(), CompileError> {
        // Allocate scratch registers for formula slot + carried locals
        let formula_reg = SCRATCH_REGS[(self.next_scratch as usize) % SCRATCH_REGS.len()];
        self.next_scratch += 1;

        let mut carried_regs = Vec::new();
        for _ in 0..inits.len() {
            let r = SCRATCH_REGS[(self.next_scratch as usize) % SCRATCH_REGS.len()];
            self.next_scratch += 1;
            carried_regs.push(r);
        }

        // Compile init values and store into carried registers
        for (i, &init) in inits.iter().enumerate() {
            self.emit_formula(order, init)?;
            let val = self.pop_reg();
            if val != carried_regs[i] {
                self.emit(ebpf_mov64(carried_regs[i], val));
            }
        }

        // Initialize formula_reg to 0 (placeholder)
        self.emit(ebpf_mov64_imm(formula_reg, 0));

        // Build loop subject
        let saved_subject = self.subject.clone();
        for &cl in carried_regs.iter() {
            self.subject.insert(0, cl);
        }
        self.subject.insert(0, formula_reg);

        // Save loop state
        let prev_loop = self.loop_state.take();
        let header_idx = self.insn_idx();
        self.loop_state = Some(EbpfLoopState {
            carried: carried_regs,
            formula_reg,
            header_idx,
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

        // Walk cons chain: skip formula slot, extract carried values
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
                self.emit(ebpf_mov64(carried_reg, val));
            }
            cur = tail;
        }

        // JA back to loop header (negative offset)
        let current_idx = self.insn_idx();
        let offset = (ls.header_idx as i32) - (current_idx as i32) - 1;
        self.emit(ebpf_ja(offset as i16));

        // Push dummy result (unreachable)
        self.push_reg();
        Ok(())
    }

    fn emit_branch<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (test, yes, no) = body_triple(order, body)?;
        self.emit_formula(order, test)?;
        let test_reg = self.pop_reg();

        // JEQ test_reg, 0, +offset → jump to yes (nox: 0=yes)
        let jeq_idx = self.insn_idx();
        self.emit(0); // placeholder

        // no branch
        self.emit_formula(order, no)?;
        let no_reg = self.pop_reg();
        let result = self.push_reg();
        if no_reg != result { self.emit(ebpf_mov64(result, no_reg)); }
        let ja_idx = self.insn_idx();
        self.emit(0); // placeholder JA (skip yes)
        self.pop_reg();

        // yes label
        let yes_idx = self.insn_idx();
        let jeq_off = (yes_idx - jeq_idx - 1) as i16;
        let jeq = ebpf_jeq_imm(test_reg, 0, jeq_off);
        self.code[jeq_idx * 8..(jeq_idx + 1) * 8].copy_from_slice(&jeq.to_le_bytes());

        self.emit_formula(order, yes)?;
        let yes_reg = self.pop_reg();
        let result2 = self.push_reg();
        if yes_reg != result2 { self.emit(ebpf_mov64(result2, yes_reg)); }

        // end label
        let end_idx = self.insn_idx();
        let ja_off = (end_idx - ja_idx - 1) as i16;
        let ja = ebpf_ja(ja_off);
        self.code[ja_idx * 8..(ja_idx + 1) * 8].copy_from_slice(&ja.to_le_bytes());

        Ok(())
    }

    fn emit_add<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();

        // dst = ra (save original)
        self.emit(ebpf_mov64(dst, ra));
        // dst += rb
        self.emit(ebpf_add64(dst, rb));
        // Goldilocks: if dst < ra (overflow): dst += 0xFFFFFFFF
        // eBPF: JGE dst, ra, +2 (skip correction)
        self.emit(ebpf_jge(dst, ra, 2));
        self.emit_load_imm64(0, 0xFFFF_FFFF); // r0 temp (2 insns for LD_IMM64)
        self.emit(ebpf_add64(dst, 0));
        // if dst >= P: dst -= P
        self.emit_load_imm64(0, P);
        self.emit(ebpf_jlt(dst, 0, 1)); // skip sub if dst < P
        self.emit(ebpf_sub64(dst, 0));

        Ok(())
    }

    fn emit_sub<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();

        // if ra >= rb: dst = ra - rb
        // else: dst = P - rb + ra
        self.emit(ebpf_mov64(dst, ra));
        self.emit(ebpf_jge(ra, rb, 4)); // skip to simple sub
        // underflow path: dst = P - rb + ra
        self.emit_load_imm64(0, P); // 2 insns
        self.emit(ebpf_mov64(dst, 0));
        self.emit(ebpf_sub64(dst, rb));
        self.emit(ebpf_add64(dst, ra));
        let skip_count = 1; // skip the simple sub
        self.emit(ebpf_ja(skip_count));
        // simple path
        self.emit(ebpf_sub64(dst, rb));

        Ok(())
    }

    fn emit_mul<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();

        // eBPF: no MULHU. Schoolbook 32×32 decomposition.
        // a = (ah << 32) | al, b = (bh << 32) | bl
        // a*b = ah*bh*2^64 + (ah*bl + al*bh)*2^32 + al*bl
        // lo = lower 64 bits, hi = upper 64 bits
        //
        // Use BPF stack for spill: [r10 - 8] = ah, [r10 - 16] = bh
        // Registers: r0 = temp, dst = result, ra/rb = inputs

        let mask32: i32 = -1; // 0xFFFFFFFF as i32 (AND mask)

        // Save al, ah, bl, bh to stack
        // [r10-8] = al = ra & 0xFFFFFFFF
        self.emit(ebpf_mov64(0, ra));
        self.emit(ebpf_and64_imm(0, mask32));
        self.emit(ebpf_stx64(10, 0, -8));  // [r10-8] = al

        // [r10-16] = ah = ra >> 32
        self.emit(ebpf_mov64(0, ra));
        self.emit(ebpf_rsh64_imm(0, 32));
        self.emit(ebpf_stx64(10, 0, -16)); // [r10-16] = ah

        // [r10-24] = bl = rb & 0xFFFFFFFF
        self.emit(ebpf_mov64(0, rb));
        self.emit(ebpf_and64_imm(0, mask32));
        self.emit(ebpf_stx64(10, 0, -24)); // [r10-24] = bl

        // [r10-32] = bh = rb >> 32
        self.emit(ebpf_mov64(0, rb));
        self.emit(ebpf_rsh64_imm(0, 32));
        self.emit(ebpf_stx64(10, 0, -32)); // [r10-32] = bh

        // lo = al * bl (fits in 64 bits, both 32-bit)
        self.emit(ebpf_ldx64(dst, 10, -8));   // dst = al
        self.emit(ebpf_ldx64(0, 10, -24));     // r0 = bl
        self.emit(ebpf_mul64(dst, 0));          // dst = al * bl = lo_lo
        self.emit(ebpf_stx64(10, dst, -40));    // [r10-40] = lo_lo

        // mid1 = ah * bl
        self.emit(ebpf_ldx64(dst, 10, -16));   // dst = ah
        self.emit(ebpf_ldx64(0, 10, -24));     // r0 = bl
        self.emit(ebpf_mul64(dst, 0));          // dst = ah*bl

        // hi = mid1 >> 32
        self.emit(ebpf_mov64(0, dst));          // r0 = mid1
        self.emit(ebpf_rsh64_imm(0, 32));
        self.emit(ebpf_stx64(10, 0, -48));     // [r10-48] = hi (partial)

        // lo = lo_lo + (mid1 & mask) << 32
        self.emit(ebpf_and64_imm(dst, mask32)); // dst = mid1 & mask
        self.emit(ebpf_lsh64_imm(dst, 32));     // dst = (mid1 & mask) << 32
        self.emit(ebpf_ldx64(0, 10, -40));      // r0 = lo_lo
        self.emit(ebpf_add64(dst, 0));           // dst = lo_lo + mid1_lo<<32
        self.emit(ebpf_stx64(10, dst, -40));     // [r10-40] = lo (updated)

        // carry: if dst < r0 (overflow)
        self.emit(ebpf_jge(dst, 0, 3));         // skip if no carry
        self.emit(ebpf_ldx64(0, 10, -48));
        self.emit(ebpf_add64_imm(0, 1));
        self.emit(ebpf_stx64(10, 0, -48));      // hi += 1

        // mid2 = al * bh
        self.emit(ebpf_ldx64(dst, 10, -8));     // dst = al
        self.emit(ebpf_ldx64(0, 10, -32));      // r0 = bh
        self.emit(ebpf_mul64(dst, 0));           // dst = al*bh

        // hi += mid2 >> 32
        self.emit(ebpf_mov64(0, dst));
        self.emit(ebpf_rsh64_imm(0, 32));
        self.emit(ebpf_ldx64(ra, 10, -48));     // ra = hi (reuse ra, done with it)
        self.emit(ebpf_add64(ra, 0));            // hi += mid2_hi
        self.emit(ebpf_stx64(10, ra, -48));

        // lo += (mid2 & mask) << 32
        self.emit(ebpf_and64_imm(dst, mask32));
        self.emit(ebpf_lsh64_imm(dst, 32));
        self.emit(ebpf_ldx64(0, 10, -40));       // r0 = lo
        self.emit(ebpf_add64(dst, 0));            // dst = lo + mid2_lo<<32
        self.emit(ebpf_stx64(10, dst, -40));

        // carry
        self.emit(ebpf_jge(dst, 0, 3));
        self.emit(ebpf_ldx64(0, 10, -48));
        self.emit(ebpf_add64_imm(0, 1));
        self.emit(ebpf_stx64(10, 0, -48));

        // hi += ah * bh
        self.emit(ebpf_ldx64(dst, 10, -16));     // ah
        self.emit(ebpf_ldx64(0, 10, -32));        // bh
        self.emit(ebpf_mul64(dst, 0));             // ah*bh
        self.emit(ebpf_ldx64(0, 10, -48));        // hi
        self.emit(ebpf_add64(0, dst));             // hi += ah*bh
        // r0 = hi, [r10-40] = lo

        // Reduce: result = lo + hi*(2^32-1) mod P
        // hi_shifted = hi << 32
        self.emit(ebpf_mov64(dst, 0));             // dst = hi (save)
        self.emit(ebpf_lsh64_imm(0, 32));          // r0 = hi << 32
        self.emit(ebpf_ldx64(ra, 10, -40));        // ra = lo
        self.emit(ebpf_add64(ra, 0));               // ra = lo + hi<<32
        self.emit(ebpf_sub64(ra, dst));              // ra = ra - hi
        self.emit(ebpf_mov64(dst, ra));              // dst = result

        // Final: if dst >= P: dst -= P
        self.emit_load_imm64(0, P);
        self.emit(ebpf_jlt(dst, 0, 1));
        self.emit(ebpf_sub64(dst, 0));

        Ok(())
    }

    fn emit_eq<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();
        // nox: 0 if equal, 1 if not
        self.emit(ebpf_mov64_imm(dst, 1)); // assume not equal
        self.emit(ebpf_jne(ra, rb, 1));    // skip if not equal
        self.emit(ebpf_mov64_imm(dst, 0)); // equal
        Ok(())
    }

    fn emit_lt<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();
        // nox: 0 if a<b, 1 if a>=b
        self.emit(ebpf_mov64_imm(dst, 1)); // assume a>=b
        self.emit(ebpf_jge(ra, rb, 1));    // skip if a>=b
        self.emit(ebpf_mov64_imm(dst, 0)); // a<b
        Ok(())
    }

    fn emit_xor<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();
        self.emit(ebpf_mov64(dst, ra));
        self.emit(ebpf_xor64(dst, rb));
        Ok(())
    }

    fn emit_and<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();
        self.emit(ebpf_mov64(dst, ra));
        self.emit(ebpf_and64(dst, rb));
        Ok(())
    }

    fn emit_not<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        self.emit_formula(order, body)?;
        let ra = self.pop_reg();
        let dst = self.push_reg();
        self.emit(ebpf_mov64(dst, ra));
        self.emit(ebpf_xor64_imm(dst, -1));
        self.emit(ebpf_and64_imm(dst, 0xFFFF_FFFFu32 as i32));
        Ok(())
    }

    fn emit_shl<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();
        self.emit(ebpf_mov64(dst, ra));
        self.emit(ebpf_lsh64(dst, rb));
        self.emit(ebpf_and64_imm(dst, 0xFFFF_FFFFu32 as i32));
        Ok(())
    }

    fn emit_load_imm64(&mut self, dst: u8, val: u64) {
        // LD_IMM64: 2 instructions (16 bytes)
        let lo = (val & 0xFFFF_FFFF) as u32;
        let hi = ((val >> 32) & 0xFFFF_FFFF) as u32;
        let insn1: u64 = 0x18 | ((dst as u64 & 0xF) << 8) | ((lo as u64) << 32);
        let insn2: u64 = (hi as u64) << 32;
        self.emit(insn1);
        self.emit(insn2);
    }
}

// ── eBPF instruction builders ────────────────────────────────────

fn ebpf_insn(opcode: u8, dst: u8, src: u8, off: i16, imm: i32) -> u64 {
    (opcode as u64)
        | (((dst & 0xF) | ((src & 0xF) << 4)) as u64) << 8
        | ((off as u16 as u64) << 16)
        | ((imm as u32 as u64) << 32)
}

fn ebpf_add64(dst: u8, src: u8) -> u64 { ebpf_insn(0x0F, dst, src, 0, 0) }
fn ebpf_sub64(dst: u8, src: u8) -> u64 { ebpf_insn(0x1F, dst, src, 0, 0) }
fn ebpf_mul64(dst: u8, src: u8) -> u64 { ebpf_insn(0x2F, dst, src, 0, 0) }
fn ebpf_and64(dst: u8, src: u8) -> u64 { ebpf_insn(0x5F, dst, src, 0, 0) }
fn ebpf_xor64(dst: u8, src: u8) -> u64 { ebpf_insn(0xAF, dst, src, 0, 0) }
fn ebpf_lsh64(dst: u8, src: u8) -> u64 { ebpf_insn(0x6F, dst, src, 0, 0) }
fn ebpf_mov64(dst: u8, src: u8) -> u64 { ebpf_insn(0xBF, dst, src, 0, 0) }

fn ebpf_add64_imm(dst: u8, imm: i32) -> u64 { ebpf_insn(0x07, dst, 0, 0, imm) }
fn ebpf_and64_imm(dst: u8, imm: i32) -> u64 { ebpf_insn(0x57, dst, 0, 0, imm) }
fn ebpf_xor64_imm(dst: u8, imm: i32) -> u64 { ebpf_insn(0xA7, dst, 0, 0, imm) }
fn ebpf_mov64_imm(dst: u8, imm: i32) -> u64 { ebpf_insn(0xB7, dst, 0, 0, imm) }
fn ebpf_lsh64_imm(dst: u8, imm: u32) -> u64 { ebpf_insn(0x67, dst, 0, 0, imm as i32) }
fn ebpf_rsh64_imm(dst: u8, imm: u32) -> u64 { ebpf_insn(0x77, dst, 0, 0, imm as i32) }

fn ebpf_stx64(dst: u8, src: u8, off: i16) -> u64 { ebpf_insn(0x7B, dst, src, off, 0) }
fn ebpf_ldx64(dst: u8, src: u8, off: i16) -> u64 { ebpf_insn(0x79, dst, src, off, 0) }

fn ebpf_jeq_imm(dst: u8, imm: i32, off: i16) -> u64 { ebpf_insn(0x15, dst, 0, off, imm) }
fn ebpf_jne(dst: u8, src: u8, off: i16) -> u64 { ebpf_insn(0x5D, dst, src, off, 0) }
fn ebpf_jge(dst: u8, src: u8, off: i16) -> u64 { ebpf_insn(0x3D, dst, src, off, 0) }
fn ebpf_jlt(dst: u8, src: u8, off: i16) -> u64 { ebpf_insn(0xAD, dst, src, off, 0) }
fn ebpf_ja(off: i16) -> u64 { ebpf_insn(0x05, 0, 0, off, 0) }

fn ebpf_exit() -> u64 { ebpf_insn(0x95, 0, 0, 0, 0) }
