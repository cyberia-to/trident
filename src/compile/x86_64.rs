//! x86-64 code emitter for nox formulas
//!
//! Hand-emitted machine code. Microsoft x64 ABI (Windows):
//! args in rcx, rdx, r8, r9 (max 4 register params), return in rax.
//! Scratch: r10-r15 (6 regs). rax/rdx used for MUL implicit operands.
//!
//! For System V (Linux/Mac x86-64): args in rdi, rsi, rdx, rcx, r8, r9.
//! Pass `sysv: true` to use System V ABI instead.
//!
//! Has MUL for unsigned 64×64→128 (RDX:RAX), same as ARM64 UMULH.




use nox::noun::{Order, NounId};
use super::{CompileError, formula_parts, body_pair, body_triple, atom_u64, axis_to_param,
            detect_loop_setup, detect_back_edge};

const P: u64 = 0xFFFF_FFFF_0000_0001;

/// Microsoft x64 ABI param registers: rcx(1), rdx(2), r8(8), r9(9)
const WIN64_PARAMS: [u8; 4] = [1, 2, 8, 9];
/// System V ABI param registers: rdi(7), rsi(6), rdx(2), rcx(1), r8(8), r9(9)
const SYSV_PARAMS: [u8; 6] = [7, 6, 2, 1, 8, 9];

/// Stack offset for param N beyond register params.
/// Win64: first stack param at [rsp + 40] (32 shadow + 8 ret addr)
/// SysV:  first stack param at [rsp + 8] (8 ret addr)
const WIN64_STACK_BASE: i32 = 40;
const SYSV_STACK_BASE: i32 = 8;

pub fn compile_to_x86_64<const N: usize>(
    order: &Order<N>, formula: NounId, num_params: u32,
) -> Result<Vec<u8>, CompileError> {
    compile_impl(order, formula, num_params, &WIN64_PARAMS, WIN64_STACK_BASE)
}

pub fn compile_to_x86_64_sysv<const N: usize>(
    order: &Order<N>, formula: NounId, num_params: u32,
) -> Result<Vec<u8>, CompileError> {
    compile_impl(order, formula, num_params, &SYSV_PARAMS, SYSV_STACK_BASE)
}

fn compile_impl<const N: usize>(
    order: &Order<N>, formula: NounId, num_params: u32,
    abi_params: &[u8], stack_base: i32,
) -> Result<Vec<u8>, CompileError> {
    let mut e = X64Emitter::new(num_params, abi_params, stack_base);
    e.emit_formula(order, formula)?;
    let result = e.pop_reg();
    if result != 0 { e.emit_mov(0, result); }
    e.code.push(0xC3); // RET
    Ok(e.code)
}

// Scratch: r10(10), r11(11), r12(12), r13(13), r14(14), r15(15)
const SCRATCH: [u8; 6] = [10, 11, 12, 13, 14, 15];

/// A subject entry: register or stack location.
#[derive(Clone, Copy)]
enum Loc {
    Reg(u8),
    /// Stack offset from rsp: [rsp + off]
    Stack(i32),
}

#[derive(Clone)]
struct X64LoopState {
    carried: Vec<u8>,         // scratch registers holding carried locals
    formula_reg: u8,          // register for formula slot
    header_offset: usize,     // byte offset of loop header in code
}

struct X64Emitter {
    code: Vec<u8>,
    reg_stack: Vec<u8>,
    next_scratch: u8,
    subject: Vec<Loc>,
    loop_state: Option<X64LoopState>,
}

impl X64Emitter {
    fn new(num_params: u32, abi_params: &[u8], stack_base: i32) -> Self {
        let reg_count = abi_params.len() as u32;
        // Build subject: last param = depth 0 (head)
        let subject: Vec<Loc> = (0..num_params).rev().map(|i| {
            if i < reg_count {
                Loc::Reg(abi_params[i as usize])
            } else {
                // Stack param: offset from rsp
                Loc::Stack(stack_base + ((i - reg_count) as i32) * 8)
            }
        }).collect();
        Self { code: Vec::with_capacity(512), reg_stack: Vec::new(), next_scratch: 0, subject, loop_state: None }
    }

    fn push_reg(&mut self) -> u8 {
        let reg = SCRATCH[(self.next_scratch as usize) % SCRATCH.len()];
        self.next_scratch += 1;
        self.reg_stack.push(reg);
        reg
    }

    fn pop_reg(&mut self) -> u8 {
        self.next_scratch -= 1;
        self.reg_stack.pop().unwrap_or(SCRATCH[0])
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
        let loc = self.subject[depth as usize];
        let dst = self.push_reg();
        match loc {
            Loc::Reg(src) => self.emit_mov(dst, src),
            Loc::Stack(off) => self.emit_load_rsp(dst, off),
        }
        Ok(())
    }

    fn emit_quote<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let val = atom_u64(order, body)?;
        let dst = self.push_reg();
        self.emit_mov_imm64(dst, val);
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
        self.subject.insert(0, Loc::Reg(val_reg));
        let result = self.emit_formula(order, body_formula);
        self.subject.remove(0);
        result
    }

    fn emit_loop<const N: usize>(
        &mut self, order: &Order<N>, loop_body: NounId, inits: &[NounId],
    ) -> Result<(), CompileError> {
        // Allocate scratch registers for formula slot + carried locals
        let formula_reg = SCRATCH[(self.next_scratch as usize) % SCRATCH.len()];
        self.next_scratch += 1;

        let mut carried_regs = Vec::new();
        for _ in 0..inits.len() {
            let r = SCRATCH[(self.next_scratch as usize) % SCRATCH.len()];
            self.next_scratch += 1;
            carried_regs.push(r);
        }

        // Compile init values and store into carried registers
        for (i, &init) in inits.iter().enumerate() {
            self.emit_formula(order, init)?;
            let val = self.pop_reg();
            if val != carried_regs[i] { self.emit_mov(carried_regs[i], val); }
        }

        // Initialize formula_reg to 0 (placeholder)
        self.emit_mov_imm64(formula_reg, 0);

        // Build loop subject
        let saved_subject = self.subject.clone();
        for &cl in carried_regs.iter() {
            self.subject.insert(0, Loc::Reg(cl));
        }
        self.subject.insert(0, Loc::Reg(formula_reg));

        // Save loop state
        let prev_loop = self.loop_state.take();
        let header_offset = self.code.len();
        self.loop_state = Some(X64LoopState {
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
            if val != carried_reg { self.emit_mov(carried_reg, val); }
            cur = tail;
        }

        // JMP back to loop header (near jump, rel32)
        self.code.push(0xE9);
        let jmp_pos = self.code.len();
        self.code.extend_from_slice(&[0, 0, 0, 0]);
        patch_rel32(&mut self.code, jmp_pos, ls.header_offset);

        // Push dummy result on reg stack (unreachable)
        self.push_reg();
        Ok(())
    }

    fn emit_branch<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (test, yes, no) = body_triple(order, body)?;
        self.emit_formula(order, test)?;
        let test_reg = self.pop_reg();

        // test rN, rN; je yes_label (nox: 0=yes)
        self.emit_rr(0x85, test_reg, test_reg); // TEST r, r
        let je_off = self.code.len();
        self.code.extend_from_slice(&[0x0F, 0x84, 0, 0, 0, 0]); // JE placeholder

        // no path
        self.emit_formula(order, no)?;
        let no_reg = self.pop_reg();
        let result = self.push_reg();
        if no_reg != result { self.emit_mov(result, no_reg); }
        let jmp_off = self.code.len();
        self.code.extend_from_slice(&[0xE9, 0, 0, 0, 0]); // JMP placeholder
        self.pop_reg();

        // yes label
        let yes_label = self.code.len();
        patch_rel32(&mut self.code, je_off + 2, yes_label);

        self.emit_formula(order, yes)?;
        let yes_reg = self.pop_reg();
        let result2 = self.push_reg();
        if yes_reg != result2 { self.emit_mov(result2, yes_reg); }

        let end_label = self.code.len();
        patch_rel32(&mut self.code, jmp_off + 1, end_label);
        Ok(())
    }

    fn emit_add<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();

        self.emit_mov(dst, ra);
        self.emit_rr(0x01, dst, rb); // ADD dst, rb
        // Goldilocks: if carry, add 0xFFFFFFFF; if >= P, sub P
        let jnc_off = self.code.len();
        self.code.extend_from_slice(&[0x0F, 0x83, 0, 0, 0, 0]); // JAE (no carry) placeholder

        // carry: dst += 0xFFFFFFFF
        self.emit_mov_imm64(0, 0xFFFF_FFFF); // rax = 0xFFFFFFFF
        self.emit_rr(0x01, dst, 0); // ADD dst, rax

        let no_carry = self.code.len();
        patch_rel32(&mut self.code, jnc_off + 2, no_carry);

        // if dst >= P: dst -= P
        self.emit_mov_imm64(0, P);
        self.emit_rr(0x39, dst, 0); // CMP dst, rax (CMP r/m64, r64)
        let jb_off = self.code.len();
        self.code.extend_from_slice(&[0x0F, 0x82, 0, 0, 0, 0]); // JB placeholder
        self.emit_rr(0x29, dst, 0); // SUB dst, rax
        let skip = self.code.len();
        patch_rel32(&mut self.code, jb_off + 2, skip);
        Ok(())
    }

    fn emit_sub<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();

        self.emit_mov(dst, ra);
        self.emit_rr(0x39, dst, rb); // CMP dst, rb
        let jae_off = self.code.len();
        self.code.extend_from_slice(&[0x0F, 0x83, 0, 0, 0, 0]); // JAE (no borrow)

        // underflow: dst = P - rb + ra
        self.emit_mov_imm64(dst, P);
        self.emit_rr(0x29, dst, rb); // SUB dst, rb
        self.emit_rr(0x01, dst, ra); // ADD dst, ra
        let jmp_off = self.code.len();
        self.code.extend_from_slice(&[0xE9, 0, 0, 0, 0]); // JMP end

        // no borrow: dst = ra - rb
        let no_borrow = self.code.len();
        patch_rel32(&mut self.code, jae_off + 2, no_borrow);
        self.emit_mov(dst, ra);
        self.emit_rr(0x29, dst, rb); // SUB dst, rb

        let end = self.code.len();
        patch_rel32(&mut self.code, jmp_off + 1, end);
        Ok(())
    }

    fn emit_mul<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();

        // x86-64 MUL: RDX:RAX = RAX * r/m64
        self.emit_mov(0, ra);          // rax = a
        self.emit_mul_rm(rb);           // RDX:RAX = a * b → rax=lo, rdx=hi
        self.emit_mov(dst, 0);         // dst = lo

        // Reduce: result = lo + hi*(2^32-1) mod P
        // hi*(2^32-1) = (hi<<32) - hi
        // Use rax as tmp (it's free now)
        let hi = 2u8; // rdx still holds hi
        self.emit_mov(0, hi);          // rax = hi (save)
        self.emit_shl_imm(hi, 32);     // rdx = hi << 32
        self.emit_rr(0x01, dst, hi);   // dst += hi<<32 (ADD dst, rdx)

        // if carry: dst += 0xFFFFFFFF
        let jnc = self.code.len();
        self.code.extend_from_slice(&[0x0F, 0x83, 0, 0, 0, 0]); // JAE no_carry
        self.emit_mov_imm64(hi, 0xFFFF_FFFF);
        self.emit_rr(0x01, dst, hi);   // dst += 0xFFFFFFFF
        let nc_label = self.code.len();
        patch_rel32(&mut self.code, jnc + 2, nc_label);

        // dst -= original_hi (in rax)
        self.emit_rr(0x29, dst, 0);    // SUB dst, rax

        // if borrow (carry clear): dst -= 0xFFFFFFFF
        let jnb = self.code.len();
        self.code.extend_from_slice(&[0x0F, 0x83, 0, 0, 0, 0]); // JAE no_borrow
        self.emit_mov_imm64(0, 0xFFFF_FFFF);
        self.emit_rr(0x29, dst, 0);    // dst -= 0xFFFFFFFF
        let nb_label = self.code.len();
        patch_rel32(&mut self.code, jnb + 2, nb_label);

        // Final: if dst >= P: dst -= P
        self.emit_mov_imm64(0, P);
        self.emit_rr(0x39, dst, 0);    // CMP dst, rax
        let jb = self.code.len();
        self.code.extend_from_slice(&[0x0F, 0x82, 0, 0, 0, 0]); // JB skip
        self.emit_rr(0x29, dst, 0);    // SUB dst, rax
        let skip = self.code.len();
        patch_rel32(&mut self.code, jb + 2, skip);
        Ok(())
    }

    fn emit_eq<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();

        self.emit_rr(0x39, ra, rb); // CMP ra, rb
        // SETNE dst_byte; MOVZX dst, dst_byte
        // nox eq: 0 if equal, 1 if not
        self.emit_mov_imm64(dst, 1);
        let je_off = self.code.len();
        self.code.extend_from_slice(&[0x0F, 0x84, 0, 0, 0, 0]); // JE
        let jmp_off = self.code.len();
        self.code.extend_from_slice(&[0xE9, 0, 0, 0, 0]); // JMP end
        let eq_label = self.code.len();
        patch_rel32(&mut self.code, je_off + 2, eq_label);
        self.emit_mov_imm64(dst, 0);
        let end = self.code.len();
        patch_rel32(&mut self.code, jmp_off + 1, end);
        Ok(())
    }

    fn emit_lt<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();

        // nox lt: 0 if a<b, 1 if a>=b
        self.emit_rr(0x39, ra, rb); // CMP ra, rb
        self.emit_mov_imm64(dst, 1); // assume a>=b
        let jae_off = self.code.len();
        self.code.extend_from_slice(&[0x0F, 0x83, 0, 0, 0, 0]); // JAE end
        self.emit_mov_imm64(dst, 0); // a < b
        let end = self.code.len();
        patch_rel32(&mut self.code, jae_off + 2, end);
        Ok(())
    }

    fn emit_xor<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();
        self.emit_mov(dst, ra);
        self.emit_rr(0x31, dst, rb); // XOR dst, rb
        Ok(())
    }

    fn emit_and<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();
        self.emit_mov(dst, ra);
        self.emit_rr(0x21, dst, rb); // AND dst, rb
        Ok(())
    }

    fn emit_not<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        self.emit_formula(order, body)?;
        let ra = self.pop_reg();
        let dst = self.push_reg();
        self.emit_mov(dst, ra);
        // NOT dst = XOR dst, -1 then AND with 0xFFFFFFFF
        self.emit_mov_imm64(0, 0xFFFF_FFFF);
        self.emit_rr(0x31, dst, dst); // XOR dst, dst → 0... no, that zeros it
        // NOT r/m64: REX.W F7 /2
        let rex = 0x48 | ((dst >= 8) as u8);
        self.code.push(rex);
        self.code.push(0xF7);
        self.code.push(0xC0 | (2 << 3) | (dst & 7)); // ModRM /2
        // AND with mask
        self.emit_rr(0x21, dst, 0); // AND dst, rax (rax=0xFFFFFFFF from above)
        Ok(())
    }

    fn emit_shl<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();
        self.emit_mov(dst, ra);
        // SHL dst, cl: need shift amount in cl (rcx low byte)
        // Save rcx if it's a param, mov rb → rcx
        self.emit_mov(1, rb); // rcx = rb (shift amount)
        // SHL dst, cl: REX.W D3 /4
        let rex = 0x48 | ((dst >= 8) as u8);
        self.code.push(rex);
        self.code.push(0xD3);
        self.code.push(0xC0 | (4 << 3) | (dst & 7));
        // Mask to 32 bits
        self.emit_mov_imm64(0, 0xFFFF_FFFF);
        self.emit_rr(0x21, dst, 0);
        Ok(())
    }

    // ── x86-64 instruction helpers ────────────────────────────────

    /// Emit REX.W + opcode + ModRM for register-register op.
    /// opcode semantics: op r/m64, r64 — dst is r/m (rm field), src is r (reg field).
    fn emit_rr(&mut self, opcode: u8, dst: u8, src: u8) {
        let rex = 0x48 | (((src >= 8) as u8) << 2) | ((dst >= 8) as u8);
        self.code.push(rex);
        self.code.push(opcode);
        self.code.push(0xC0 | ((src & 7) << 3) | (dst & 7));
    }

    /// MOV dst, src (register to register)
    fn emit_mov(&mut self, dst: u8, src: u8) {
        if dst != src {
            self.emit_rr(0x89, dst, src); // MOV r/m64, r64
        }
    }

    /// MOV reg, imm64
    fn emit_mov_imm64(&mut self, reg: u8, val: u64) {
        if val == 0 {
            // XOR reg, reg (shorter encoding)
            self.emit_rr(0x31, reg, reg);
            return;
        }
        let rex = 0x48 | ((reg >= 8) as u8);
        self.code.push(rex);
        self.code.push(0xB8 + (reg & 7));
        self.code.extend_from_slice(&val.to_le_bytes());
    }

    /// MUL r/m64: RDX:RAX = RAX * r/m64
    fn emit_mul_rm(&mut self, rm: u8) {
        let rex = 0x48 | ((rm >= 8) as u8);
        self.code.push(rex);
        self.code.push(0xF7);
        self.code.push(0xC0 | (4 << 3) | (rm & 7)); // /4 = MUL
    }

    /// SHL reg, imm8
    fn emit_shl_imm(&mut self, reg: u8, imm: u8) {
        let rex = 0x48 | ((reg >= 8) as u8);
        self.code.push(rex);
        self.code.push(0xC1);
        self.code.push(0xC0 | (4 << 3) | (reg & 7));
        self.code.push(imm);
    }

    /// MOV reg, [rsp + offset] — load 64-bit from stack
    fn emit_load_rsp(&mut self, dst: u8, offset: i32) {
        // MOV r64, [rsp + disp32]: REX.W 8B ModRM SIB disp32
        // ModRM: mod=10 (disp32), reg=dst, rm=100 (SIB follows)
        // SIB: scale=00, index=100 (none), base=100 (rsp)
        let rex = 0x48 | (((dst >= 8) as u8) << 2);
        self.code.push(rex);
        self.code.push(0x8B);
        self.code.push(0x84 | ((dst & 7) << 3)); // ModRM: mod=10, reg=dst, rm=100
        self.code.push(0x24); // SIB: rsp base, no index
        self.code.extend_from_slice(&offset.to_le_bytes());
    }
}

fn patch_rel32(code: &mut Vec<u8>, offset: usize, target: usize) {
    let rel = (target as i32) - ((offset + 4) as i32);
    code[offset..offset + 4].copy_from_slice(&rel.to_le_bytes());
}
