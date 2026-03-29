//! RISC-V 32-bit (RV32IM) code emitter for nox formulas
//!
//! Hand-emitted machine code, no dependencies.
//! Args in a0-a7 (x10-x17), return in a0 (x10).
//! Scratch: t0-t2 (x5-x7), t3-t6 (x28-x31) = 7 regs.
//! Has MULHU for upper 32 bits of 32×32 multiply.
//!
//! Phase 1: 32-bit values only — no Goldilocks reduction.
//! Full Goldilocks field elements require 64-bit pairs (lo, hi)
//! with carry-propagating add/sub and 4-partial-product mul.
//! That is Phase 2 (register-pair mode for ESP32 IoT use cases).

use nox::noun::{Order, NounId};
use super::{CompileError, formula_parts, body_pair, body_triple, atom_u64, axis_to_param,
            detect_loop_setup, detect_back_edge};

const MAX_PARAMS: u32 = 8;

pub fn compile_to_rv32<const N: usize>(
    order: &Order<N>,
    formula: NounId,
    num_params: u32,
) -> Result<Vec<u8>, CompileError> {
    if num_params > MAX_PARAMS { return Err(CompileError::NoParams); }
    let mut e = Rv32Emitter::new(num_params);
    e.emit_formula(order, formula)?;
    let result_reg = e.pop_reg();
    // Move result to a0 (x10)
    if result_reg != 10 {
        e.emit_u32(rv32_addi(10, result_reg, 0)); // mv a0, result
    }
    e.emit_u32(rv32_ret());
    Ok(e.code)
}

#[derive(Clone)]
struct Rv32LoopState {
    carried: Vec<u8>,         // registers holding carried locals
    formula_reg: u8,          // register for formula slot
    header_offset: usize,     // byte offset of loop header in code
}

struct Rv32Emitter {
    code: Vec<u8>,
    reg_stack: Vec<u8>,
    next_scratch: u8,
    subject: Vec<u8>,
    loop_state: Option<Rv32LoopState>,
}

// Scratch: t0(x5), t1(x6), t2(x7), t3(x28), t4(x29), t5(x30), t6(x31)
const SCRATCH_REGS: [u8; 7] = [5, 6, 7, 28, 29, 30, 31];

impl Rv32Emitter {
    fn new(num_params: u32) -> Self {
        // Args: a0=x10..a7=x17. Subject: last param=depth 0 (head)
        let subject: Vec<u8> = (0..num_params).rev().map(|i| (10 + i) as u8).collect();
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

    fn emit_u32(&mut self, insn: u32) {
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

    fn emit_axis<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let addr = atom_u64(order, body)?;
        let depth = axis_to_param(addr)?;
        if (depth as usize) >= self.subject.len() { return Err(CompileError::NoParams); }
        let src = self.subject[depth as usize];
        let dst = self.push_reg();
        self.emit_u32(rv32_addi(dst, src, 0)); // mv dst, src
        Ok(())
    }

    fn emit_quote<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let val = atom_u64(order, body)?;
        let dst = self.push_reg();
        self.emit_load_imm32(dst, val as u32);
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
                self.emit_u32(rv32_addi(carried_regs[i], val, 0));
            }
        }

        // Initialize formula_reg to 0 (placeholder)
        self.emit_u32(rv32_addi(formula_reg, 0, 0));

        // Build loop subject
        let saved_subject = self.subject.clone();
        for &cl in carried_regs.iter() {
            self.subject.insert(0, cl);
        }
        self.subject.insert(0, formula_reg);

        // Save loop state
        let prev_loop = self.loop_state.take();
        let header_offset = self.code.len();
        self.loop_state = Some(Rv32LoopState {
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
                self.emit_u32(rv32_addi(carried_reg, val, 0));
            }
            cur = tail;
        }

        // JAL x0, offset (unconditional jump back to loop header)
        let current = self.code.len();
        let offset = ls.header_offset as i32 - current as i32;
        self.emit_u32(rv32_jal(0, offset));

        // Push dummy result (unreachable)
        self.push_reg();
        Ok(())
    }

    fn emit_branch<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (test, yes, no) = body_triple(order, body)?;
        self.emit_formula(order, test)?;
        let test_reg = self.pop_reg();

        // BEQ test_reg, x0, yes_branch (nox: 0=yes)
        let beq_off = self.code.len();
        self.emit_u32(0); // placeholder

        // no branch
        self.emit_formula(order, no)?;
        let no_reg = self.pop_reg();
        let result = self.push_reg();
        if no_reg != result { self.emit_u32(rv32_addi(result, no_reg, 0)); }
        let jal_off = self.code.len();
        self.emit_u32(0); // placeholder JAL (skip yes)
        self.pop_reg();

        // yes label
        let yes_label = self.code.len();
        let beq_imm = (yes_label as i32) - (beq_off as i32);
        let beq = rv32_beq(test_reg, 0, beq_imm);
        self.code[beq_off..beq_off + 4].copy_from_slice(&beq.to_le_bytes());

        self.emit_formula(order, yes)?;
        let yes_reg = self.pop_reg();
        let result2 = self.push_reg();
        if yes_reg != result2 { self.emit_u32(rv32_addi(result2, yes_reg, 0)); }

        let end_label = self.code.len();
        let jal_imm = (end_label as i32) - (jal_off as i32);
        let jal = rv32_jal(0, jal_imm); // JAL x0, skip (unconditional jump)
        self.code[jal_off..jal_off + 4].copy_from_slice(&jal.to_le_bytes());

        Ok(())
    }

    fn emit_add<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();

        // Phase 1: 32-bit add, no Goldilocks reduction
        self.emit_u32(rv32_add(dst, ra, rb));
        Ok(())
    }

    fn emit_sub<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();

        // Phase 1: 32-bit sub, no Goldilocks reduction
        self.emit_u32(rv32_sub(dst, ra, rb));
        Ok(())
    }

    fn emit_mul<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();

        // Phase 1: lo 32 bits of 32×32 multiply
        // MULHU available for upper 32 bits if needed (Phase 2)
        self.emit_u32(rv32_mul(dst, ra, rb));
        Ok(())
    }

    fn emit_eq<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();
        // dst = (ra != rb) ? 1 : 0
        self.emit_u32(rv32_sub(dst, ra, rb));      // dst = ra - rb
        self.emit_u32(rv32_sltu(dst, 0, dst));      // dst = (0 < dst) = (dst != 0)
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
        // SLTU dst, ra, rb → dst = (ra < rb unsigned) ? 1 : 0
        // Then flip: dst = 1 - dst
        self.emit_u32(rv32_sltu(dst, ra, rb));
        self.emit_u32(rv32_xori(dst, dst, 1));
        Ok(())
    }

    fn emit_xor<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();
        self.emit_u32(rv32_xor(dst, ra, rb));
        Ok(())
    }

    fn emit_and<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();
        self.emit_u32(rv32_and(dst, ra, rb));
        Ok(())
    }

    fn emit_not<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        self.emit_formula(order, body)?;
        let ra = self.pop_reg();
        let dst = self.push_reg();
        self.emit_u32(rv32_xori(dst, ra, -1));    // NOT = XOR with -1 (all bits set)
        Ok(())
    }

    fn emit_shl<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();
        // SLL uses low 5 bits of rs2 as shamt on RV32
        self.emit_u32(rv32_sll(dst, ra, rb));
        Ok(())
    }

    // ── helpers ───────────────────────────────────────────────────

    /// Load a 32-bit immediate into register rd.
    /// Uses LUI + ADDI sequence for values that don't fit in 12 bits.
    fn emit_load_imm32(&mut self, rd: u8, val: u32) {
        if val == 0 {
            self.emit_u32(rv32_addi(rd, 0, 0));
            return;
        }
        if val < 2048 {
            self.emit_u32(rv32_addi(rd, 0, val as i32));
            return;
        }
        // Sign-extended 12-bit lower
        let lower = ((val & 0xFFF) as i32) << 20 >> 20;
        // Upper 20 bits, adjusted for sign extension of lower
        let upper = (val.wrapping_add(0x800) >> 12) & 0xFFFFF;
        if upper != 0 {
            self.emit_u32(rv32_lui(rd, upper));
            if lower != 0 { self.emit_u32(rv32_addi(rd, rd, lower)); }
        } else {
            self.emit_u32(rv32_addi(rd, 0, lower));
        }
    }
}

// ── RISC-V 32-bit instruction encoders ──────────────────────────────

// R-type: opcode 0x33, same encoding as RV64 base integer
fn rv32_add(rd: u8, rs1: u8, rs2: u8) -> u32 {
    0x33 | ((rd as u32) << 7) | ((rs1 as u32) << 15) | ((rs2 as u32) << 20)
}

fn rv32_sub(rd: u8, rs1: u8, rs2: u8) -> u32 {
    0x33 | ((rd as u32) << 7) | ((rs1 as u32) << 15) | ((rs2 as u32) << 20) | (0x20 << 25)
}

// M extension: MUL — low 32 bits of rs1 × rs2
fn rv32_mul(rd: u8, rs1: u8, rs2: u8) -> u32 {
    0x33 | ((rd as u32) << 7) | ((rs1 as u32) << 15) | ((rs2 as u32) << 20) | (1 << 25)
}

// M extension: MULHU — upper 32 bits of unsigned rs1 × rs2
#[allow(dead_code)]
fn rv32_mulhu(rd: u8, rs1: u8, rs2: u8) -> u32 {
    0x33 | ((rd as u32) << 7) | (3 << 12) | ((rs1 as u32) << 15) | ((rs2 as u32) << 20) | (1 << 25)
}

fn rv32_and(rd: u8, rs1: u8, rs2: u8) -> u32 {
    0x33 | ((rd as u32) << 7) | (7 << 12) | ((rs1 as u32) << 15) | ((rs2 as u32) << 20)
}

#[allow(dead_code)]
fn rv32_or(rd: u8, rs1: u8, rs2: u8) -> u32 {
    0x33 | ((rd as u32) << 7) | (6 << 12) | ((rs1 as u32) << 15) | ((rs2 as u32) << 20)
}

fn rv32_xor(rd: u8, rs1: u8, rs2: u8) -> u32 {
    0x33 | ((rd as u32) << 7) | (4 << 12) | ((rs1 as u32) << 15) | ((rs2 as u32) << 20)
}

fn rv32_sll(rd: u8, rs1: u8, rs2: u8) -> u32 {
    0x33 | ((rd as u32) << 7) | (1 << 12) | ((rs1 as u32) << 15) | ((rs2 as u32) << 20)
}

fn rv32_sltu(rd: u8, rs1: u8, rs2: u8) -> u32 {
    0x33 | ((rd as u32) << 7) | (3 << 12) | ((rs1 as u32) << 15) | ((rs2 as u32) << 20)
}

// I-type: opcode 0x13
fn rv32_addi(rd: u8, rs1: u8, imm: i32) -> u32 {
    let imm12 = (imm as u32) & 0xFFF;
    0x13 | ((rd as u32) << 7) | ((rs1 as u32) << 15) | (imm12 << 20)
}

fn rv32_xori(rd: u8, rs1: u8, imm: i32) -> u32 {
    let imm12 = (imm as u32) & 0xFFF;
    0x13 | ((rd as u32) << 7) | (4 << 12) | ((rs1 as u32) << 15) | (imm12 << 20)
}

// SLLI: shamt is 5 bits on RV32 (0-31), not 6 bits like RV64
#[allow(dead_code)]
fn rv32_slli(rd: u8, rs1: u8, shamt: u8) -> u32 {
    0x13 | ((rd as u32) << 7) | (1 << 12) | ((rs1 as u32) << 15) | (((shamt & 0x1F) as u32) << 20)
}

// U-type: LUI — same encoding as RV64
fn rv32_lui(rd: u8, imm20: u32) -> u32 {
    0x37 | ((rd as u32) << 7) | ((imm20 & 0xFFFFF) << 12)
}

// RET = JALR x0, x1, 0
fn rv32_ret() -> u32 {
    0x67 | (1 << 15)
}

// J-type: JAL — same encoding
fn rv32_jal(rd: u8, imm: i32) -> u32 {
    let imm_20 = ((imm >> 20) & 1) as u32;
    let imm_10_1 = ((imm >> 1) & 0x3FF) as u32;
    let imm_11 = ((imm >> 11) & 1) as u32;
    let imm_19_12 = ((imm >> 12) & 0xFF) as u32;
    0x6F | ((rd as u32) << 7) | (imm_19_12 << 12) | (imm_11 << 20) | (imm_10_1 << 21) | (imm_20 << 31)
}

// B-type: branch encoding — same as RV64
fn rv32_branch(rs1: u8, rs2: u8, imm: i32, funct3: u32) -> u32 {
    let imm_12 = ((imm >> 12) & 1) as u32;
    let imm_10_5 = ((imm >> 5) & 0x3F) as u32;
    let imm_4_1 = ((imm >> 1) & 0xF) as u32;
    let imm_11 = ((imm >> 11) & 1) as u32;
    0x63 | (imm_11 << 7) | (imm_4_1 << 8) | (funct3 << 12)
        | ((rs1 as u32) << 15) | ((rs2 as u32) << 20) | (imm_10_5 << 25) | (imm_12 << 31)
}

fn rv32_beq(rs1: u8, rs2: u8, imm: i32) -> u32 { rv32_branch(rs1, rs2, imm, 0) }
#[allow(dead_code)]
fn rv32_bltu(rs1: u8, rs2: u8, imm: i32) -> u32 { rv32_branch(rs1, rs2, imm, 6) }
#[allow(dead_code)]
fn rv32_bgeu(rs1: u8, rs2: u8, imm: i32) -> u32 { rv32_branch(rs1, rs2, imm, 7) }
