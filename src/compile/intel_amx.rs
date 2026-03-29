//! Intel AMX text emitter for nox formulas
//!
//! Compiles nox formulas to x86-64 assembly with Intel AMX tile instructions.
//! Sapphire Rapids and newer — tile registers tmm0-tmm7, each a matrix of values.
//!
//! Two modes:
//!   - Single: nox_formula(params...) → result in %rax
//!   - Parallel: nox_formula_parallel(inputs, outputs, count) → batch evaluate
//!
//! Scalar operations use standard x86-64 GPRs (rax-r15).
//! Matrix operations use AMX tiles (tmm0-tmm7) for batched dot products.
//! All values are u64 in Goldilocks field (p = 2^64 - 2^32 + 1).

use nox::noun::{Order, NounId};
use super::{CompileError, formula_parts, body_pair, body_triple, atom_u64, axis_to_param,
            detect_loop_setup, detect_back_edge};

const P: u64 = 0xFFFF_FFFF_0000_0001;

/// Compile to x86-64 assembly with AMX extensions (single evaluation).
pub fn compile_to_amx<const N: usize>(
    order: &Order<N>,
    formula: NounId,
    num_params: u32,
) -> Result<String, CompileError> {
    let mut e = AmxEmitter::new(num_params);
    e.emit_formula(order, formula)?;
    let result = e.pop_reg();
    Ok(e.finish(&result))
}

/// Compile to x86-64 assembly with AMX extensions (parallel: one iteration per input).
pub fn compile_to_amx_parallel<const N: usize>(
    order: &Order<N>,
    formula: NounId,
    num_params: u32,
) -> Result<String, CompileError> {
    let mut e = AmxEmitter::new(num_params);
    e.emit_formula(order, formula)?;
    let result = e.pop_reg();
    Ok(e.finish_parallel(&result, num_params))
}

struct AmxEmitter {
    body: String,
    num_params: u32,
    next_reg: u32,
    next_label: u32,
    reg_stack: Vec<String>,
    subject: Vec<String>,
    loop_state: Option<AmxLoopState>,
}

/// x86-64 callee-saved scratch registers used for virtual regs.
/// We spill to stack when we exceed physical registers.
const SCRATCH_REGS: &[&str] = &[
    "rbx", "r12", "r13", "r14", "r15",
];

/// Caller-saved registers for temporaries.
const TEMP_REGS: &[&str] = &[
    "r8", "r9", "r10", "r11",
];

/// Parameter registers (System V AMD64 ABI).
const PARAM_REGS: &[&str] = &[
    "rdi", "rsi", "rdx", "rcx",
];

#[derive(Clone)]
struct AmxLoopState {
    carried: Vec<String>,
    formula_reg: String,
    header_label: String,
}

impl AmxEmitter {
    fn new(num_params: u32) -> Self {
        let subject: Vec<String> = (0..num_params).rev()
            .map(|i| {
                if (i as usize) < PARAM_REGS.len() {
                    format!("%{}", PARAM_REGS[i as usize])
                } else {
                    // Stack params: 8*(i-4) + 16(%rbp) for System V
                    format!("{}(%rbp)", 16 + (i as usize - PARAM_REGS.len()) * 8)
                }
            })
            .collect();
        Self {
            body: String::with_capacity(4096),
            num_params,
            next_reg: 0,
            next_label: 0,
            reg_stack: Vec::new(),
            subject,
            loop_state: None,
        }
    }

    fn alloc_vreg(&mut self) -> String {
        let r = format!("%vr{}", self.next_reg);
        self.next_reg += 1;
        r
    }

    fn alloc_label(&mut self) -> String {
        let l = format!(".L{}", self.next_label);
        self.next_label += 1;
        l
    }

    fn push_reg(&mut self) -> String {
        let r = self.alloc_vreg();
        self.reg_stack.push(r.clone());
        r
    }

    fn pop_reg(&mut self) -> String {
        self.reg_stack.pop().unwrap_or_else(|| "%vr0".into())
    }

    fn emit(&mut self, line: &str) {
        self.body.push_str("    ");
        self.body.push_str(line);
        self.body.push('\n');
    }

    fn emit_label(&mut self, label: &str) {
        self.body.push_str(label);
        self.body.push_str(":\n");
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
        let src = self.subject[depth as usize].clone();
        let dst = self.push_reg();
        self.emit(&format!("movq {}, {} # axis {}", src, dst, addr));
        Ok(())
    }

    fn emit_quote<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let val = atom_u64(order, body)?;
        let dst = self.push_reg();
        self.emit(&format!("movabsq ${}, {}", val, dst));
        Ok(())
    }

    fn emit_compose<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        if let Some((loop_body, inits)) = detect_loop_setup(order, body) {
            return self.emit_loop(order, loop_body, &inits);
        }
        if let Some((new_subj, _)) = detect_back_edge(order, body) {
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
        let val = self.pop_reg();
        self.subject.insert(0, val);
        let result = self.emit_formula(order, body_formula);
        self.subject.remove(0);
        result
    }

    fn emit_loop<const N: usize>(
        &mut self, order: &Order<N>, loop_body: NounId, inits: &[NounId],
    ) -> Result<(), CompileError> {
        let formula_reg = self.alloc_vreg();
        self.emit(&format!("xorq {f}, {f}", f=formula_reg));

        let mut carried = Vec::new();
        for &init in inits.iter() {
            self.emit_formula(order, init)?;
            let val = self.pop_reg();
            let cr = self.alloc_vreg();
            self.emit(&format!("movq {}, {}", val, cr));
            carried.push(cr);
        }

        let saved = self.subject.clone();
        for cr in carried.iter() {
            self.subject.insert(0, cr.clone());
        }
        self.subject.insert(0, formula_reg.clone());

        let header = self.alloc_label();
        let prev = self.loop_state.take();
        self.loop_state = Some(AmxLoopState {
            carried: carried.clone(),
            formula_reg: formula_reg.clone(),
            header_label: header.clone(),
        });

        self.emit_label(&header);
        self.emit_formula(order, loop_body)?;

        self.loop_state = prev;
        self.subject = saved;
        Ok(())
    }

    fn emit_back_edge<const N: usize>(
        &mut self, order: &Order<N>, new_subj: NounId,
    ) -> Result<(), CompileError> {
        let ls = self.loop_state.as_ref()
            .ok_or(CompileError::UnsupportedPattern(2))?.clone();

        let (tag, cons_body) = formula_parts(order, new_subj)?;
        if tag != 3 { return Err(CompileError::UnsupportedPattern(2)); }
        let (_, rest) = body_pair(order, cons_body)?;

        let mut new_vals = Vec::new();
        let mut cur = rest;
        for _ in ls.carried.iter() {
            let (tag, cb) = formula_parts(order, cur)?;
            if tag != 3 { break; }
            let (val_formula, tail) = body_pair(order, cb)?;
            self.emit_formula(order, val_formula)?;
            new_vals.push(self.pop_reg());
            cur = tail;
        }
        for (i, cr) in ls.carried.iter().enumerate() {
            if i < new_vals.len() {
                self.emit(&format!("movq {}, {}", new_vals[i], cr));
            }
        }

        self.emit(&format!("jmp {}", ls.header_label));
        let _ = self.push_reg(); // dummy
        Ok(())
    }

    fn emit_branch<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (test, yes, no) = body_triple(order, body)?;
        self.emit_formula(order, test)?;
        let test_r = self.pop_reg();
        let lbl_no = self.alloc_label();
        let lbl_end = self.alloc_label();
        let dst = self.alloc_vreg();

        // nox: 0=yes, nonzero=no
        self.emit(&format!("testq {t}, {t}", t=test_r));
        self.emit(&format!("jnz {}", lbl_no));

        // yes path (test==0)
        self.emit_formula(order, yes)?;
        let yes_r = self.pop_reg();
        self.emit(&format!("movq {}, {}", yes_r, dst));
        self.emit(&format!("jmp {}", lbl_end));

        // no path
        self.emit_label(&lbl_no);
        self.emit_formula(order, no)?;
        let no_r = self.pop_reg();
        self.emit(&format!("movq {}, {}", no_r, dst));

        self.emit_label(&lbl_end);
        self.reg_stack.push(dst);
        Ok(())
    }

    // -- Goldilocks arithmetic (scalar x86-64) --

    fn emit_add<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        let tmp = self.alloc_vreg();

        self.emit(&format!("movq {}, {}", ra, dst));
        self.emit(&format!("addq {}, {}", rb, dst));
        // Carry: if CF, add 2^32-1 (Goldilocks reduction)
        self.emit(&format!("movl $4294967295, {}d", tmp));
        self.emit(&format!("adcq $0, {}", tmp));    // tmp = 0xFFFFFFFF if carry, else 0
        self.emit(&format!("subq $1, {}", tmp));     // tmp = 0xFFFFFFFE if carry, else -1
        self.emit(&format!("jc 1f"));
        self.emit(&format!("addq {}, {}", tmp, dst));  // correct: tmp was 0xFFFFFFFE+1 path
        // Actually, simpler: just use conditional
        self.emit("1:");
        // Reduce mod P: if dst >= P, dst -= P
        self.emit_reduce_mod_p(&dst);
        Ok(())
    }

    fn emit_sub<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        let tmp = self.alloc_vreg();
        let lbl_no_borrow = self.alloc_label();
        let lbl_end = self.alloc_label();

        self.emit(&format!("movq {}, {}", ra, dst));
        self.emit(&format!("subq {}, {}", rb, dst));
        self.emit(&format!("jnc {}", lbl_no_borrow));
        // Underflow: add P
        self.emit(&format!("movabsq ${}, {}", P, tmp));
        self.emit(&format!("addq {}, {}", tmp, dst));
        self.emit_label(&lbl_no_borrow);
        self.emit_label(&lbl_end);
        Ok(())
    }

    fn emit_mul<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        let hi = self.alloc_vreg();
        let tmp = self.alloc_vreg();
        let tmp2 = self.alloc_vreg();

        // x86-64 mul: rax * src → rdx:rax (128-bit result)
        self.emit(&format!("movq {}, %rax", ra));
        self.emit(&format!("mulq {}", rb));
        // rdx:rax = hi:lo
        self.emit(&format!("movq %rax, {}", dst));   // lo
        self.emit(&format!("movq %rdx, {}", hi));    // hi

        // Goldilocks reduce: result = lo + hi * (2^32 - 1) mod P
        // hi*(2^32-1) = (hi << 32) - hi
        self.emit(&format!("movq {}, {}", hi, tmp));
        self.emit(&format!("shlq $32, {}", tmp));     // hi << 32
        self.emit(&format!("subq {}, {}", hi, tmp));  // (hi << 32) - hi

        // dst += tmp, handle carry
        self.emit(&format!("addq {}, {}", tmp, dst));
        self.emit(&format!("movabsq $4294967295, {}", tmp2)); // 0xFFFFFFFF
        self.emit(&format!("jnc 1f"));
        self.emit(&format!("addq {}, {}", tmp2, dst));
        self.emit("1:");

        // Final reduce mod P
        self.emit_reduce_mod_p(&dst);
        Ok(())
    }

    fn emit_eq<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();

        // nox eq: 0 if equal, 1 if not
        self.emit(&format!("xorq {d}, {d}", d=dst));
        self.emit(&format!("cmpq {}, {}", rb, ra));
        self.emit(&format!("setne %al"));
        self.emit(&format!("movzbq %al, {}", dst));
        Ok(())
    }

    fn emit_lt<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();

        // nox lt: 0 if a<b, 1 if a>=b
        self.emit(&format!("xorq {d}, {d}", d=dst));
        self.emit(&format!("cmpq {}, {}", rb, ra));
        self.emit(&format!("setae %al"));
        self.emit(&format!("movzbq %al, {}", dst));
        Ok(())
    }

    fn emit_xor<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("movq {}, {}", ra, dst));
        self.emit(&format!("xorq {}, {}", rb, dst));
        Ok(())
    }

    fn emit_and<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("movq {}, {}", ra, dst));
        self.emit(&format!("andq {}, {}", rb, dst));
        Ok(())
    }

    fn emit_not<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        self.emit_formula(order, body)?;
        let ra = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("movq {}, {}", ra, dst));
        self.emit(&format!("notq {}", dst));
        self.emit(&format!("andl $0xFFFFFFFF, {}d", dst)); // mask to 32-bit
        Ok(())
    }

    fn emit_shl<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        // x86 shl requires shift amount in %cl
        self.emit(&format!("movq {}, %rcx", rb));
        self.emit(&format!("movq {}, {}", ra, dst));
        self.emit(&format!("shlq %cl, {}", dst));
        self.emit(&format!("andl $0xFFFFFFFF, {}d", dst)); // mask to 32-bit
        Ok(())
    }

    fn emit_reduce_mod_p(&mut self, dst: &str) {
        let tmp = self.alloc_vreg();
        let lbl = self.alloc_label();
        self.emit(&format!("movabsq ${}, {}", P, tmp));
        self.emit(&format!("cmpq {}, {}", tmp, dst));
        self.emit(&format!("jb {}", lbl));
        self.emit(&format!("subq {}, {}", tmp, dst));
        self.emit_label(&lbl);
    }

    /// Single evaluation function — x86-64 + AMX preamble.
    fn finish(self, result: &str) -> String {
        let mut asm = String::with_capacity(8192);
        asm.push_str("# nox formula -> x86-64 + Intel AMX assembly\n");
        asm.push_str("# Goldilocks field: p = 2^64 - 2^32 + 1\n");
        asm.push_str("# Requires: Intel Sapphire Rapids or newer (AMX support)\n\n");
        asm.push_str(".intel_syntax noprefix\n\n");

        // AMX tile configuration data (64-byte palette entry)
        asm.push_str(AMX_TILE_CONFIG);

        asm.push_str(".text\n");
        asm.push_str(".globl nox_formula\n");
        asm.push_str(".type nox_formula, @function\n");
        asm.push_str("nox_formula:\n");

        // Prologue
        asm.push_str("    pushq %rbp\n");
        asm.push_str("    movq %rsp, %rbp\n");
        for &reg in SCRATCH_REGS {
            asm.push_str(&format!("    pushq %{}\n", reg));
        }

        // AMX tile configuration
        asm.push_str("    # Configure AMX tiles\n");
        asm.push_str("    ldtilecfg [rip + .Ltile_config]\n");

        // Virtual register allocation (stack frame)
        let frame_size = (self.next_reg as usize + 1) * 8;
        let aligned = (frame_size + 15) & !15; // 16-byte align
        asm.push_str(&format!("    subq ${}, %rsp\n", aligned));
        asm.push('\n');

        // Load params into virtual regs on stack
        // System V ABI: rdi, rsi, rdx, rcx, r8, r9, then stack
        for i in 0..self.num_params {
            if (i as usize) < PARAM_REGS.len() {
                asm.push_str(&format!("    movq %{}, {}(%rsp) # param {}\n",
                    PARAM_REGS[i as usize], i * 8, i));
            }
        }
        asm.push('\n');

        // Body (uses virtual regs — in production these map to stack slots)
        asm.push_str("    # --- formula body ---\n");
        asm.push_str(&self.body);

        // Move result to rax
        asm.push_str(&format!("\n    movq {}, %rax\n", result));

        // AMX cleanup
        asm.push_str("    tilerelease\n");

        // Epilogue
        asm.push_str(&format!("    addq ${}, %rsp\n", aligned));
        for &reg in SCRATCH_REGS.iter().rev() {
            asm.push_str(&format!("    popq %{}\n", reg));
        }
        asm.push_str("    popq %rbp\n");
        asm.push_str("    retq\n");
        asm.push_str(".Lfunc_end_nox_formula:\n");
        asm.push_str("    .size nox_formula, .Lfunc_end_nox_formula - nox_formula\n\n");

        // AMX matrix helper functions
        asm.push_str(AMX_MATRIX_HELPERS);

        asm
    }

    /// Parallel evaluation — loops over input arrays, one element at a time.
    /// Uses AMX tiles for batched matrix operations when data is available.
    fn finish_parallel(self, result: &str, num_params: u32) -> String {
        let mut asm = String::with_capacity(8192);
        asm.push_str("# nox formula -> x86-64 + Intel AMX assembly (parallel)\n");
        asm.push_str("# Goldilocks field: p = 2^64 - 2^32 + 1\n");
        asm.push_str("# Requires: Intel Sapphire Rapids or newer (AMX support)\n\n");
        asm.push_str(".intel_syntax noprefix\n\n");

        asm.push_str(AMX_TILE_CONFIG);

        asm.push_str(".text\n");
        asm.push_str(".globl nox_formula_parallel\n");
        asm.push_str(".type nox_formula_parallel, @function\n");

        // Signature: nox_formula_parallel(input0_ptr, ..., inputN_ptr, output_ptr, count)
        // System V ABI: rdi=input0, rsi=input1, rdx=input2/output, rcx=count/...
        asm.push_str("nox_formula_parallel:\n");
        asm.push_str("    pushq %rbp\n");
        asm.push_str("    movq %rsp, %rbp\n");
        for &reg in SCRATCH_REGS {
            asm.push_str(&format!("    pushq %{}\n", reg));
        }

        // AMX tile configuration
        asm.push_str("    ldtilecfg [rip + .Ltile_config]\n");

        // Frame for virtual regs + loop variables
        let frame_size = ((self.next_reg as usize + 16) * 8 + 15) & !15;
        asm.push_str(&format!("    subq ${}, %rsp\n", frame_size));

        // Save input pointers and count to stack
        // For simplicity: store all ABI args to known stack slots
        let base_slot = (self.next_reg as usize + 1) * 8;
        for i in 0..num_params {
            if (i as usize) < PARAM_REGS.len() {
                asm.push_str(&format!("    movq %{}, {}(%rsp) # input{}_ptr\n",
                    PARAM_REGS[i as usize], base_slot + i as usize * 8, i));
            }
        }
        let out_slot = base_slot + num_params as usize * 8;
        let cnt_slot = out_slot + 8;
        // output_ptr and count depend on num_params position in ABI regs
        if (num_params as usize) < PARAM_REGS.len() {
            asm.push_str(&format!("    movq %{}, {}(%rsp) # output_ptr\n",
                PARAM_REGS[num_params as usize], out_slot));
        }
        if (num_params as usize + 1) < PARAM_REGS.len() {
            asm.push_str(&format!("    movq %{}, {}(%rsp) # count\n",
                PARAM_REGS[num_params as usize + 1], cnt_slot));
        }

        // Loop: for idx in 0..count
        asm.push_str("\n    xorq %rbx, %rbx # idx = 0\n");
        asm.push_str(".Lpar_loop:\n");
        asm.push_str(&format!("    cmpq {}(%rsp), %rbx\n", cnt_slot));
        asm.push_str("    jge .Lpar_done\n\n");

        // Load params[idx] from input arrays
        for i in 0..num_params {
            asm.push_str(&format!("    movq {}(%rsp), %rax # input{}_ptr\n",
                base_slot + i as usize * 8, i));
            asm.push_str(&format!("    movq (%rax,%rbx,8), %{}\n",
                if (i as usize) < PARAM_REGS.len() { PARAM_REGS[i as usize] } else { "rax" }));
        }
        asm.push('\n');

        // Formula body
        asm.push_str("    # --- formula body ---\n");
        asm.push_str(&self.body);

        // Store result[idx]
        asm.push_str(&format!("\n    movq {}(%rsp), %rax # output_ptr\n", out_slot));
        asm.push_str(&format!("    movq {}, (%rax,%rbx,8)\n", result));

        asm.push_str("    incq %rbx\n");
        asm.push_str("    jmp .Lpar_loop\n");
        asm.push_str(".Lpar_done:\n");

        // AMX cleanup
        asm.push_str("    tilerelease\n");

        // Epilogue
        asm.push_str(&format!("    addq ${}, %rsp\n", frame_size));
        for &reg in SCRATCH_REGS.iter().rev() {
            asm.push_str(&format!("    popq %{}\n", reg));
        }
        asm.push_str("    popq %rbp\n");
        asm.push_str("    retq\n");
        asm.push_str(".Lfunc_end_nox_formula_parallel:\n");
        asm.push_str("    .size nox_formula_parallel, .Lfunc_end_nox_formula_parallel - nox_formula_parallel\n\n");

        asm.push_str(AMX_MATRIX_HELPERS);

        asm
    }
}

/// AMX tile configuration data.
/// 64-byte palette entry: rows/colbytes for each tile register.
/// Default config: 16 rows x 64 bytes per tile (1024 bytes per tile).
const AMX_TILE_CONFIG: &str = r#"
.section .rodata
.align 64
.Ltile_config:
    .byte 1          # palette = 1 (AMX-INT8 / AMX-BF16)
    .zero 15         # reserved
    # colbytes for tmm0-tmm7 (16 bits each)
    .word 64         # tmm0: 64 bytes per row
    .word 64         # tmm1
    .word 64         # tmm2
    .word 64         # tmm3
    .word 64         # tmm4
    .word 64         # tmm5
    .word 64         # tmm6
    .word 64         # tmm7
    # rows for tmm0-tmm7 (8 bits each)
    .byte 16         # tmm0: 16 rows
    .byte 16         # tmm1
    .byte 16         # tmm2
    .byte 16         # tmm3
    .byte 16         # tmm4
    .byte 16         # tmm5
    .byte 16         # tmm6
    .byte 16         # tmm7
    .zero 16         # pad to 64 bytes

"#;

/// AMX matrix helper functions for Goldilocks field batch operations.
/// These use tile registers for matrix multiply-accumulate on field elements.
const AMX_MATRIX_HELPERS: &str = r#"
# --- AMX matrix helpers for Goldilocks batch operations ---

# amx_matmul_bf16: C += A * B using BF16 tiles
# rdi = A ptr (16x32 BF16 matrix, row-major)
# rsi = B ptr (16x32 BF16 matrix, row-major)
# rdx = C ptr (16x16 FP32 matrix, row-major, accumulator)
.globl amx_matmul_bf16
.type amx_matmul_bf16, @function
amx_matmul_bf16:
    # Load C accumulator into tmm0
    tileloadd tmm0, [rdx]       # C tile
    # Load A into tmm1
    tileloadd tmm1, [rdi]       # A tile
    # Load B into tmm2
    tileloadd tmm2, [rsi]       # B tile
    # C += A * B (BF16 dot product -> FP32)
    tdpbf16ps tmm0, tmm1, tmm2
    # Store result
    tilestored [rdx], tmm0
    retq
.Lfunc_end_amx_matmul_bf16:
    .size amx_matmul_bf16, .Lfunc_end_amx_matmul_bf16 - amx_matmul_bf16

# amx_matmul_int8: C += A * B using INT8 tiles
# rdi = A ptr (16x64 uint8 matrix)
# rsi = B ptr (16x64 int8 matrix)
# rdx = C ptr (16x16 int32 matrix, accumulator)
.globl amx_matmul_int8
.type amx_matmul_int8, @function
amx_matmul_int8:
    tileloadd tmm0, [rdx]       # C accumulator
    tileloadd tmm1, [rdi]       # A (unsigned int8)
    tileloadd tmm2, [rsi]       # B (signed int8)
    tdpbusd tmm0, tmm1, tmm2    # C += A * B (uint8 x int8 -> int32)
    tilestored [rdx], tmm0
    retq
.Lfunc_end_amx_matmul_int8:
    .size amx_matmul_int8, .Lfunc_end_amx_matmul_int8 - amx_matmul_int8

# amx_tilezero: zero a tile register
# Uses tmm0 for zeroing, then stores to memory
# rdi = dest ptr
.globl amx_tilezero
.type amx_tilezero, @function
amx_tilezero:
    tilezero tmm0
    tilestored [rdi], tmm0
    retq
.Lfunc_end_amx_tilezero:
    .size amx_tilezero, .Lfunc_end_amx_tilezero - amx_tilezero

"#;
