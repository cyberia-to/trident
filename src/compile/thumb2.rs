//! ARM Cortex-M (Thumb-2) code emitter for nox formulas
//!
//! Targets STM32, RP2040, and other embedded Cortex-M processors.
//! Thumb-2: mix of 16-bit and 32-bit instructions, little-endian
//! halfword order for 32-bit encodings.
//!
//! Phase 1: 32-bit field values only (no Goldilocks, no native 64-bit).
//!
//! Register allocation (AAPCS32):
//!   r0-r3:   function args + return (max 4 params)
//!   r4-r11:  scratch registers for intermediates
//!   r13=sp, r14=lr, r15=pc (never touched)
//!
//! All arithmetic is plain 32-bit wrapping — no modular reduction.
//! 32-bit Thumb-2 instructions: emit hw1 (little-endian u16), then
//! hw2 (little-endian u16).



use nox::noun::{Order, NounId};
use super::{CompileError, formula_parts, body_pair, body_triple, atom_u64, axis_to_param,
            detect_loop_setup, detect_back_edge};

const MAX_PARAMS: u32 = 4; // AAPCS32: r0-r3

/// Compile a nox formula to Thumb-2 machine code.
pub fn compile_to_thumb2<const N: usize>(
    order: &Order<N>,
    formula: NounId,
    num_params: u32,
) -> Result<Vec<u8>, CompileError> {
    if num_params > MAX_PARAMS {
        return Err(CompileError::NoParams);
    }
    let mut emitter = Thumb2Emitter::new(num_params);
    emitter.emit_formula(order, formula)?;
    let result_reg = emitter.pop_reg();
    if result_reg != 0 {
        emitter.emit_mov_reg(0, result_reg);
    }
    emitter.emit_bx_lr();
    Ok(emitter.code)
}

#[derive(Clone)]
struct Thumb2LoopState {
    carried: Vec<u8>,
    formula_reg: u8,
    header_offset: usize,
}

struct Thumb2Emitter {
    code: Vec<u8>,
    reg_stack: Vec<u8>,
    next_scratch: u8,
    subject: Vec<u8>,
    loop_state: Option<Thumb2LoopState>,
}

// Scratch: r4, r5, r6, r7, r8, r9, r10, r11
const SCRATCH_BASE: u8 = 4;
const SCRATCH_COUNT: u8 = 8;

impl Thumb2Emitter {
    fn new(num_params: u32) -> Self {
        // Args: r0-r3. Subject: last param = depth 0 (head).
        let subject: Vec<u8> = (0..num_params).rev().map(|i| i as u8).collect();
        Self {
            code: Vec::with_capacity(512),
            reg_stack: Vec::with_capacity(16),
            next_scratch: 0,
            subject,
            loop_state: None,
        }
    }

    fn push_reg(&mut self) -> u8 {
        let reg = SCRATCH_BASE + (self.next_scratch % SCRATCH_COUNT);
        self.next_scratch += 1;
        self.reg_stack.push(reg);
        reg
    }

    fn pop_reg(&mut self) -> u8 {
        self.next_scratch -= 1;
        self.reg_stack.pop().unwrap_or(SCRATCH_BASE)
    }

    /// Emit a 32-bit Thumb-2 instruction as two little-endian halfwords.
    fn emit_thumb32(&mut self, hw1: u16, hw2: u16) {
        self.code.extend_from_slice(&hw1.to_le_bytes());
        self.code.extend_from_slice(&hw2.to_le_bytes());
    }

    /// Emit a 16-bit Thumb instruction.
    fn emit_thumb16(&mut self, insn: u16) {
        self.code.extend_from_slice(&insn.to_le_bytes());
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

    fn emit_quote<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let val = atom_u64(order, body)?;
        let dst = self.push_reg();
        self.emit_mov_imm32(dst, val as u32);
        Ok(())
    }

    fn emit_compose<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        if let Some((loop_body, inits)) = detect_loop_setup(order, body) {
            return self.emit_loop(order, loop_body, &inits);
        }

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
        let formula_reg = SCRATCH_BASE + (self.next_scratch % SCRATCH_COUNT);
        self.next_scratch += 1;

        let mut carried_regs = Vec::new();
        for _ in 0..inits.len() {
            let r = SCRATCH_BASE + (self.next_scratch % SCRATCH_COUNT);
            self.next_scratch += 1;
            carried_regs.push(r);
        }

        for (i, &init) in inits.iter().enumerate() {
            self.emit_formula(order, init)?;
            let val = self.pop_reg();
            if val != carried_regs[i] {
                self.emit_mov_reg(carried_regs[i], val);
            }
        }

        // Initialize formula_reg to 0
        self.emit_mov_imm32(formula_reg, 0);

        let saved_subject = self.subject.clone();
        for &cl in carried_regs.iter() {
            self.subject.insert(0, cl);
        }
        self.subject.insert(0, formula_reg);

        let prev_loop = self.loop_state.take();
        let header_offset = self.code.len();
        self.loop_state = Some(Thumb2LoopState {
            carried: carried_regs,
            formula_reg,
            header_offset,
        });

        self.emit_formula(order, loop_body)?;

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

        // B.W (unconditional branch) back to loop header
        let current = self.code.len();
        // Offset from start of this B.W instruction to header_offset
        // Thumb branch offset is calculated from PC = instruction address + 4
        let offset = ls.header_offset as i32 - (current as i32 + 4);
        self.emit_branch_w(offset);

        self.push_reg();
        Ok(())
    }

    fn emit_branch<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (test, yes, no) = body_triple(order, body)?;

        self.emit_formula(order, test)?;
        let test_reg = self.pop_reg();

        // CMP.W test_reg, #0
        self.emit_cmp_imm(test_reg, 0);

        // BEQ.W yes_label (nox: 0 = yes)
        let beq_off = self.code.len();
        self.emit_thumb32(0, 0); // placeholder

        // no branch (test != 0)
        self.emit_formula(order, no)?;
        let no_reg = self.pop_reg();
        let result = self.push_reg();
        if no_reg != result {
            self.emit_mov_reg(result, no_reg);
        }
        let b_off = self.code.len();
        self.emit_thumb32(0, 0); // placeholder B.W (skip yes)
        self.pop_reg();

        // yes label
        let yes_label = self.code.len();
        // Patch BEQ.W: offset from BEQ instruction to yes_label
        let beq_offset = yes_label as i32 - (beq_off as i32 + 4);
        let (hw1, hw2) = thumb2_bcond_w(0x0, beq_offset); // cond=0 = EQ
        self.code[beq_off..beq_off + 2].copy_from_slice(&hw1.to_le_bytes());
        self.code[beq_off + 2..beq_off + 4].copy_from_slice(&hw2.to_le_bytes());

        self.emit_formula(order, yes)?;
        let yes_reg = self.pop_reg();
        let result2 = self.push_reg();
        if yes_reg != result2 {
            self.emit_mov_reg(result2, yes_reg);
        }

        // end label — patch B.W
        let end_label = self.code.len();
        let b_offset = end_label as i32 - (b_off as i32 + 4);
        let (hw1, hw2) = thumb2_b_w(b_offset);
        self.code[b_off..b_off + 2].copy_from_slice(&hw1.to_le_bytes());
        self.code[b_off + 2..b_off + 4].copy_from_slice(&hw2.to_le_bytes());

        Ok(())
    }

    fn emit_add<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();
        // ADD.W Rd, Rn, Rm
        self.emit_thumb32(
            0xEB00 | (ra as u16),
            ((dst as u16) << 8) | (rb as u16),
        );
        Ok(())
    }

    fn emit_sub<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();
        // SUB.W Rd, Rn, Rm
        self.emit_thumb32(
            0xEBA0 | (ra as u16),
            ((dst as u16) << 8) | (rb as u16),
        );
        Ok(())
    }

    fn emit_mul<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();
        // MUL Rd, Rn, Rm = MLA with Ra=0xF (no accumulate)
        // hw1 = 0xFB00 | Rn, hw2 = 0xF000 | (Rd << 8) | Rm
        self.emit_thumb32(
            0xFB00 | (ra as u16),
            0xF000 | ((dst as u16) << 8) | (rb as u16),
        );
        Ok(())
    }

    fn emit_eq<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();

        // SUB.W dst, ra, rb (dst = ra - rb; nonzero if not equal)
        self.emit_thumb32(
            0xEBA0 | (ra as u16),
            ((dst as u16) << 8) | (rb as u16),
        );

        // Compare dst to 0: CMP.W dst, #0
        self.emit_cmp_imm(dst, 0);

        // IT NE: if not equal, set dst=1; else dst=0
        // MOV dst, #0 first
        self.emit_mov_imm32(dst, 0);
        // BEQ.W over the MOV #1 (skip 4 bytes = one 32-bit instruction)
        let beq_off = self.code.len();
        self.emit_thumb32(0, 0); // placeholder BEQ.W
        // MOV dst, #1
        self.emit_mov_imm32(dst, 1);
        let end = self.code.len();
        let beq_offset = end as i32 - (beq_off as i32 + 4);
        let (hw1, hw2) = thumb2_bcond_w(0x0, beq_offset); // EQ
        self.code[beq_off..beq_off + 2].copy_from_slice(&hw1.to_le_bytes());
        self.code[beq_off + 2..beq_off + 4].copy_from_slice(&hw2.to_le_bytes());

        Ok(())
    }

    fn emit_lt<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();

        // CMP.W ra, rb
        self.emit_thumb32(
            0xEBB0 | (ra as u16),
            0x0F00 | (rb as u16),
        );

        // nox: 0 if a<b, 1 if a>=b
        // MOV dst, #1 (assume a >= b)
        self.emit_mov_imm32(dst, 1);
        // BHS.W skip (if carry set = unsigned higher or same, keep dst=1)
        let bhs_off = self.code.len();
        self.emit_thumb32(0, 0); // placeholder BHS.W (cond=0x2 = CS/HS)
        // a < b: MOV dst, #0
        self.emit_mov_imm32(dst, 0);
        let end = self.code.len();
        let bhs_offset = end as i32 - (bhs_off as i32 + 4);
        let (hw1, hw2) = thumb2_bcond_w(0x2, bhs_offset); // CS/HS
        self.code[bhs_off..bhs_off + 2].copy_from_slice(&hw1.to_le_bytes());
        self.code[bhs_off + 2..bhs_off + 4].copy_from_slice(&hw2.to_le_bytes());

        Ok(())
    }

    fn emit_xor<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();
        // EOR.W Rd, Rn, Rm
        self.emit_thumb32(
            0xEA80 | (ra as u16),
            ((dst as u16) << 8) | (rb as u16),
        );
        Ok(())
    }

    fn emit_and<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();
        // AND.W Rd, Rn, Rm
        self.emit_thumb32(
            0xEA00 | (ra as u16),
            ((dst as u16) << 8) | (rb as u16),
        );
        Ok(())
    }

    fn emit_not<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        self.emit_formula(order, body)?;
        let ra = self.pop_reg();
        let dst = self.push_reg();
        // MVN.W Rd, Rm (bitwise NOT)
        // hw1 = 0xEA6F, hw2 = (Rd << 8) | Rm
        self.emit_thumb32(
            0xEA6F,
            ((dst as u16) << 8) | (ra as u16),
        );
        Ok(())
    }

    fn emit_shl<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();
        // LSL.W Rd, Rn, Rm
        self.emit_thumb32(
            0xFA00 | (ra as u16),
            0xF000 | ((dst as u16) << 8) | (rb as u16),
        );
        Ok(())
    }

    // ── instruction helpers ───────────────────────────────────────

    /// MOV Rd, Rm (16-bit Thumb encoding, works for all r0-r15)
    fn emit_mov_reg(&mut self, dst: u8, src: u8) {
        if dst == src { return; }
        // 16-bit MOV Rd, Rm: 0x4600 | (D << 7) | (Rm << 3) | (Rd & 7)
        // D = high bit of Rd (bit 3)
        let d_bit = (dst >> 3) & 1;
        let insn = 0x4600u16
            | ((d_bit as u16) << 7)
            | ((src as u16 & 0xF) << 3)
            | (dst as u16 & 0x7);
        self.emit_thumb16(insn);
    }

    /// Load a 32-bit immediate into register using MOVW + optional MOVT.
    fn emit_mov_imm32(&mut self, reg: u8, val: u32) {
        let lo16 = val & 0xFFFF;
        self.emit_movw(reg, lo16 as u16);
        if val > 0xFFFF {
            let hi16 = (val >> 16) & 0xFFFF;
            self.emit_movt(reg, hi16 as u16);
        }
    }

    /// MOVW Rd, #imm16 — load lower 16 bits, zero upper 16.
    /// Encoding: hw1 = 0xF240 | (i << 10) | imm4
    ///           hw2 = (imm3 << 12) | (Rd << 8) | imm8
    /// where imm16 = (imm4 << 12) | (i << 11) | (imm3 << 8) | imm8
    fn emit_movw(&mut self, rd: u8, imm16: u16) {
        let imm8 = imm16 & 0xFF;
        let imm3 = (imm16 >> 8) & 0x7;
        let i = (imm16 >> 11) & 0x1;
        let imm4 = (imm16 >> 12) & 0xF;

        let hw1 = 0xF240u16 | ((i as u16) << 10) | imm4;
        let hw2 = ((imm3 as u16) << 12) | ((rd as u16) << 8) | imm8;
        self.emit_thumb32(hw1, hw2);
    }

    /// MOVT Rd, #imm16 — load upper 16 bits, keep lower 16.
    /// Same bit layout as MOVW but hw1 base = 0xF2C0.
    fn emit_movt(&mut self, rd: u8, imm16: u16) {
        let imm8 = imm16 & 0xFF;
        let imm3 = (imm16 >> 8) & 0x7;
        let i = (imm16 >> 11) & 0x1;
        let imm4 = (imm16 >> 12) & 0xF;

        let hw1 = 0xF2C0u16 | ((i as u16) << 10) | imm4;
        let hw2 = ((imm3 as u16) << 12) | ((rd as u16) << 8) | imm8;
        self.emit_thumb32(hw1, hw2);
    }

    /// CMP.W Rn, #imm8 — compare register to immediate.
    /// For simplicity, supports 0-255 only.
    fn emit_cmp_imm(&mut self, rn: u8, imm8: u8) {
        // CMP.W Rn, #imm8: hw1 = 0xF1B0 | Rn, hw2 = 0x0F00 | imm8
        self.emit_thumb32(
            0xF1B0 | (rn as u16),
            0x0F00 | (imm8 as u16),
        );
    }

    /// BX LR — return from function (16-bit).
    fn emit_bx_lr(&mut self) {
        self.emit_thumb16(0x4770);
    }

    /// B.W offset — unconditional 32-bit branch.
    fn emit_branch_w(&mut self, offset: i32) {
        let (hw1, hw2) = thumb2_b_w(offset);
        self.emit_thumb32(hw1, hw2);
    }
}

// ── Thumb-2 branch encoders ──────────────────────────────────────

/// Encode B.W (unconditional 32-bit branch).
/// offset is from PC (instruction address + 4) to target.
///
/// Encoding (T4): hw1 = 1111 0 S imm10, hw2 = 10 J1 1 J2 imm11
/// where: I1 = NOT(J1 XOR S), I2 = NOT(J2 XOR S)
/// target = SignExtend(S:I1:I2:imm10:imm11:0, 25)
fn thumb2_b_w(offset: i32) -> (u16, u16) {
    let s = ((offset >> 24) & 1) as u16;
    let i1 = ((offset >> 23) & 1) as u16;
    let i2 = ((offset >> 22) & 1) as u16;
    let imm10 = ((offset >> 12) & 0x3FF) as u16;
    let imm11 = ((offset >> 1) & 0x7FF) as u16;

    let j1 = (i1 ^ s) ^ 1;
    let j2 = (i2 ^ s) ^ 1;

    let hw1 = 0xF000 | (s << 10) | imm10;
    let hw2 = 0x9000 | (j1 << 13) | (j2 << 11) | imm11;
    (hw1, hw2)
}

/// Encode B<cond>.W (conditional 32-bit branch).
/// offset is from PC (instruction address + 4) to target.
///
/// Encoding (T3): hw1 = 1111 0 S cond imm6, hw2 = 10 J1 0 J2 imm11
/// target = SignExtend(S:J2:J1:imm6:imm11:0, 21)
fn thumb2_bcond_w(cond: u8, offset: i32) -> (u16, u16) {
    let s = ((offset >> 20) & 1) as u16;
    let j2 = ((offset >> 19) & 1) as u16;
    let j1 = ((offset >> 18) & 1) as u16;
    let imm6 = ((offset >> 12) & 0x3F) as u16;
    let imm11 = ((offset >> 1) & 0x7FF) as u16;

    let hw1 = 0xF000 | (s << 10) | ((cond as u16 & 0xF) << 6) | imm6;
    let hw2 = 0x8000 | (j1 << 13) | (j2 << 11) | imm11;
    (hw1, hw2)
}
