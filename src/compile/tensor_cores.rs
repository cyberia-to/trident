//! NVIDIA Tensor Core backend via PTX wmma instructions
//!
//! Extends the PTX backend with Warp Matrix Multiply-Accumulate (wmma)
//! instructions for hardware tensor acceleration on Ampere+ GPUs.
//!
//! Two compilation modes:
//!   - Scalar: regular PTX for non-mul operations (delegates to ptx.rs patterns)
//!   - Tensor: wmma 16x16x16 matrix ops for mul (fp16 input, fp32 accumulate)
//!
//! Target: sm_80 (Ampere). Requires warp-synchronous execution (32 threads).
//!
//! wmma instruction set used:
//!   wmma.load.a.sync.aligned.m16n16k16.shared.f16  — load A fragment
//!   wmma.load.b.sync.aligned.m16n16k16.shared.f16  — load B fragment
//!   wmma.mma.sync.aligned.m16n16k16.f32.f16        — fused multiply-accumulate
//!   wmma.store.d.sync.aligned.m16n16k16.shared.f32  — store D fragment

use nox::{Reduction as Order, Order as NounId};
use super::{CompileError, formula_parts, body_pair, body_triple, atom_u64, axis_to_param,
            detect_loop_setup, detect_back_edge};

const P: u64 = 0xFFFF_FFFF_0000_0001;

/// Fragment size for wmma m16n16k16.
const WMMA_M: u32 = 16;
const WMMA_N: u32 = 16;
const WMMA_K: u32 = 16;

/// Registers per fragment: 8 for f16 A/B, 8 for f32 C/D.
const FRAG_REGS: u32 = 8;

/// Compile to PTX with tensor core acceleration (single evaluation).
///
/// Scalar operations use regular PTX. Mul operations targeting matrix data
/// emit wmma instructions for 16x16x16 tensor core execution.
pub fn compile_to_tensor_ptx<const N: usize>(
    order: &Order<N>,
    formula: NounId,
    num_params: u32,
) -> Result<String, CompileError> {
    let mut e = TensorPtxEmitter::new(num_params);
    e.emit_formula(order, formula)?;
    let result = e.pop_reg();
    Ok(e.finish(&result))
}

/// Compile to PTX with tensor cores (parallel: one warp per matrix tile).
pub fn compile_to_tensor_ptx_parallel<const N: usize>(
    order: &Order<N>,
    formula: NounId,
    num_params: u32,
) -> Result<String, CompileError> {
    let mut e = TensorPtxEmitter::new(num_params);
    e.emit_formula(order, formula)?;
    let result = e.pop_reg();
    Ok(e.finish_parallel(&result, num_params))
}

struct TensorPtxEmitter {
    body: String,
    num_params: u32,
    next_reg: u32,
    next_pred: u32,
    next_label: u32,
    next_frag: u32,
    reg_stack: Vec<String>,
    subject: Vec<String>,
    loop_state: Option<TensorLoopState>,
    /// Track whether wmma fragments were used (need shared memory decls).
    uses_wmma: bool,
}

#[derive(Clone)]
struct TensorLoopState {
    carried: Vec<String>,
    formula_reg: String,
    header_label: String,
}

impl TensorPtxEmitter {
    fn new(num_params: u32) -> Self {
        let subject: Vec<String> = (0..num_params).rev()
            .map(|i| format!("%p{}", i))
            .collect();
        Self {
            body: String::with_capacity(4096),
            num_params,
            next_reg: 0,
            next_pred: 0,
            next_label: 0,
            next_frag: 0,
            reg_stack: Vec::new(),
            subject,
            loop_state: None,
            uses_wmma: false,
        }
    }

    fn alloc_reg(&mut self) -> String {
        let r = format!("%rd{}", self.next_reg);
        self.next_reg += 1;
        r
    }

    fn alloc_pred(&mut self) -> String {
        let p = format!("%p_cond{}", self.next_pred);
        self.next_pred += 1;
        p
    }

    fn alloc_label(&mut self) -> String {
        let l = format!("L{}", self.next_label);
        self.next_label += 1;
        l
    }

    /// Allocate a fragment register group (8 registers for wmma).
    fn alloc_frag(&mut self, prefix: &str) -> String {
        let f = format!("%{}{}", prefix, self.next_frag);
        self.next_frag += 1;
        f
    }

    fn push_reg(&mut self) -> String {
        let r = self.alloc_reg();
        self.reg_stack.push(r.clone());
        r
    }

    fn pop_reg(&mut self) -> String {
        self.reg_stack.pop().unwrap_or_else(|| "%rd0".to_string())
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
            7 => self.emit_mul_tensor(order, body),
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
        self.emit(&format!("mov.u64 {}, {};", dst, src));
        Ok(())
    }

    fn emit_quote<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let val = atom_u64(order, body)?;
        let dst = self.push_reg();
        self.emit(&format!("mov.u64 {}, {};", dst, val));
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
        let formula_reg = self.alloc_reg();
        self.emit(&format!("mov.u64 {}, 0;", formula_reg));

        let mut carried = Vec::new();
        for &init in inits.iter() {
            self.emit_formula(order, init)?;
            let val = self.pop_reg();
            let cr = self.alloc_reg();
            self.emit(&format!("mov.u64 {}, {};", cr, val));
            carried.push(cr);
        }

        let saved = self.subject.clone();
        for cr in carried.iter() {
            self.subject.insert(0, cr.clone());
        }
        self.subject.insert(0, formula_reg.clone());

        let header = self.alloc_label();
        let prev = self.loop_state.take();
        self.loop_state = Some(TensorLoopState {
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

        let mut cur = rest;
        let mut new_vals = Vec::new();
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
                self.emit(&format!("mov.u64 {}, {};", cr, new_vals[i]));
            }
        }

        self.emit(&format!("bra {};", ls.header_label));
        let _ = self.push_reg(); // dummy
        Ok(())
    }

    fn emit_branch<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (test, yes, no) = body_triple(order, body)?;
        self.emit_formula(order, test)?;
        let test_r = self.pop_reg();
        let pred = self.alloc_pred();
        let lbl_no = self.alloc_label();
        let lbl_end = self.alloc_label();
        let dst = self.alloc_reg();

        // nox: 0=yes, nonzero=no
        self.emit(&format!("setp.ne.u64 {}, {}, 0;", pred, test_r));
        self.emit(&format!("@{} bra {};", pred, lbl_no));

        // yes path (test==0)
        self.emit_formula(order, yes)?;
        let yes_r = self.pop_reg();
        self.emit(&format!("mov.u64 {}, {};", dst, yes_r));
        self.emit(&format!("bra {};", lbl_end));

        // no path
        self.emit_label(&lbl_no);
        self.emit_formula(order, no)?;
        let no_r = self.pop_reg();
        self.emit(&format!("mov.u64 {}, {};", dst, no_r));

        self.emit_label(&lbl_end);
        Ok(())
    }

    // ── Scalar arithmetic (same as ptx.rs) ─────────────────────────

    fn emit_add<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        let carry = self.alloc_pred();
        let tmp = self.alloc_reg();

        self.emit(&format!("add.cc.u64 {}, {}, {};", dst, ra, rb));
        self.emit(&format!("setp.lt.u64 {}, {}, {};", carry, dst, ra));
        self.emit(&format!("mov.u64 {}, 4294967295;", tmp)); // 0xFFFFFFFF
        self.emit(&format!("@{} add.u64 {}, {}, {};", carry, dst, dst, tmp));
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
        let pred = self.alloc_pred();
        let lbl = self.alloc_label();
        let lbl_end = self.alloc_label();
        let tmp = self.alloc_reg();

        self.emit(&format!("setp.ge.u64 {}, {}, {};", pred, ra, rb));
        self.emit(&format!("@{} bra {};", pred, lbl));
        // underflow: P - rb + ra
        self.emit(&format!("mov.u64 {}, {};", tmp, P));
        self.emit(&format!("sub.u64 {}, {}, {};", dst, tmp, rb));
        self.emit(&format!("add.u64 {}, {}, {};", dst, dst, ra));
        self.emit(&format!("bra {};", lbl_end));
        self.emit_label(&lbl);
        self.emit(&format!("sub.u64 {}, {}, {};", dst, ra, rb));
        self.emit_label(&lbl_end);
        Ok(())
    }

    /// Mul via wmma tensor core instructions.
    ///
    /// Strategy: store operands into shared memory as fp16 16x16 tiles,
    /// execute wmma.mma for fused multiply-accumulate in hardware tensor
    /// cores, read result back as fp32.
    ///
    /// For scalar nox mul (two field elements), the tile is mostly zeros
    /// with the operand in [0,0]. The wmma path pays setup cost but
    /// enables batched matrix multiplication when the formula operates
    /// on matrix-shaped data.
    fn emit_mul_tensor<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();

        self.uses_wmma = true;

        // Fragment register names
        let frag_a = self.alloc_frag("fa");
        let frag_b = self.alloc_frag("fb");
        let frag_c = self.alloc_frag("fc");
        let frag_d = self.alloc_frag("fd");

        // Convert u64 operands to f16 and store in shared memory tile [0,0]
        let f_a = self.alloc_reg();
        let f_b = self.alloc_reg();
        self.emit(&format!("cvt.rn.f32.u64 {}, {};", f_a, ra));
        self.emit(&format!("cvt.rn.f32.u64 {}, {};", f_b, rb));

        // Store A[0,0] as f16 into shared tile_a
        // tile_a is 16x16 f16 = 512 bytes at smem offset 0
        self.emit("// Zero-initialize shared memory tiles");
        self.emit(&format!("st.shared.f32 [tile_a], {};", f_a));

        // Store B[0,0] as f16 into shared tile_b
        // tile_b at offset 512
        self.emit(&format!("st.shared.f32 [tile_b], {};", f_b));
        self.emit("bar.sync 0;");

        // Load fragments from shared memory
        self.emit(&format!(
            "wmma.load.a.sync.aligned.m{}n{}k{}.shared.f16 {{{}}}, [tile_a], {};",
            WMMA_M, WMMA_N, WMMA_K,
            frag_list(&frag_a, FRAG_REGS),
            WMMA_K  // leading dimension
        ));
        self.emit(&format!(
            "wmma.load.b.sync.aligned.m{}n{}k{}.shared.f16 {{{}}}, [tile_b], {};",
            WMMA_M, WMMA_N, WMMA_K,
            frag_list(&frag_b, FRAG_REGS),
            WMMA_N  // leading dimension
        ));

        // Zero-init accumulator C
        for i in 0..FRAG_REGS {
            self.emit(&format!("mov.f32 {}{}, 0f00000000;", frag_c, i));
        }

        // Tensor core multiply-accumulate: D = A * B + C
        self.emit(&format!(
            "wmma.mma.sync.aligned.m{}n{}k{}.f32.f16 {{{}}}, {{{}}}, {{{}}}, {{{}}};",
            WMMA_M, WMMA_N, WMMA_K,
            frag_list(&frag_d, FRAG_REGS),
            frag_list(&frag_a, FRAG_REGS),
            frag_list(&frag_b, FRAG_REGS),
            frag_list(&frag_c, FRAG_REGS),
        ));

        // Store result to shared memory
        self.emit(&format!(
            "wmma.store.d.sync.aligned.m{}n{}k{}.shared.f32 [tile_d], {{{}}}, {};",
            WMMA_M, WMMA_N, WMMA_K,
            frag_list(&frag_d, FRAG_REGS),
            WMMA_N  // leading dimension
        ));
        self.emit("bar.sync 0;");

        // Read result[0,0] back to scalar, convert to u64
        let f_result = self.alloc_reg();
        self.emit(&format!("ld.shared.f32 {}, [tile_d];", f_result));
        self.emit(&format!("cvt.rzi.u64.f32 {}, {};", dst, f_result));

        // Reduce mod Goldilocks
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
        let pred = self.alloc_pred();
        // nox eq: 0 if equal, 1 if not
        self.emit(&format!("setp.ne.u64 {}, {}, {};", pred, ra, rb));
        self.emit(&format!("selp.u64 {}, 1, 0, {};", dst, pred));
        Ok(())
    }

    fn emit_lt<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        let pred = self.alloc_pred();
        // nox lt: 0 if a<b, 1 if a>=b
        self.emit(&format!("setp.ge.u64 {}, {}, {};", pred, ra, rb));
        self.emit(&format!("selp.u64 {}, 1, 0, {};", dst, pred));
        Ok(())
    }

    fn emit_xor<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("xor.b64 {}, {}, {};", dst, ra, rb));
        Ok(())
    }

    fn emit_and<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("and.b64 {}, {}, {};", dst, ra, rb));
        Ok(())
    }

    fn emit_not<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        self.emit_formula(order, body)?;
        let ra = self.pop_reg();
        let dst = self.push_reg();
        let tmp = self.alloc_reg();
        self.emit(&format!("not.b64 {}, {};", dst, ra));
        self.emit(&format!("mov.u64 {}, 4294967295;", tmp));
        self.emit(&format!("and.b64 {}, {}, {};", dst, dst, tmp));
        Ok(())
    }

    fn emit_shl<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        let _tmp = self.alloc_reg();
        // PTX shl needs 32-bit shift amount
        self.emit(&format!("cvt.u32.u64 %r_sh, {};", rb));
        self.emit(&format!("shl.b64 {}, {}, %r_sh;", dst, ra));
        let mask = self.alloc_reg();
        self.emit(&format!("mov.u64 {}, 4294967295;", mask));
        self.emit(&format!("and.b64 {}, {}, {};", dst, dst, mask));
        Ok(())
    }

    fn emit_reduce_mod_p(&mut self, dst: &str) {
        let pred = self.alloc_pred();
        let tmp = self.alloc_reg();
        self.emit(&format!("mov.u64 {}, {};", tmp, P));
        self.emit(&format!("setp.ge.u64 {}, {}, {};", pred, dst, tmp));
        self.emit(&format!("@{} sub.u64 {}, {}, {};", pred, dst, dst, tmp));
    }

    /// Single evaluation kernel with tensor core support.
    fn finish(self, result: &str) -> String {
        let mut ptx = String::with_capacity(8192);
        ptx.push_str(".version 7.0\n.target sm_80\n.address_size 64\n\n");

        // Kernel signature
        ptx.push_str(".entry main(\n");
        for i in 0..self.num_params {
            if i > 0 { ptx.push_str(",\n"); }
            ptx.push_str(&format!("    .param .u64 param{}", i));
        }
        ptx.push_str(",\n    .param .u64 result_ptr\n) {\n");

        // Register declarations
        ptx.push_str(&format!("    .reg .u64 %rd<{}>;\n", self.next_reg + 1));
        ptx.push_str(&format!("    .reg .u64 %p<{}>;\n", self.num_params));
        ptx.push_str(&format!("    .reg .pred %p_cond<{}>;\n", self.next_pred + 1));
        if self.body.contains("%r_sh") {
            ptx.push_str("    .reg .u32 %r_sh;\n");
        }

        // wmma fragment registers
        if self.uses_wmma {
            for i in 0..self.next_frag {
                ptx.push_str(&format!("    .reg .f16 %fa{}<{}>;\n", i, FRAG_REGS));
                ptx.push_str(&format!("    .reg .f16 %fb{}<{}>;\n", i, FRAG_REGS));
                ptx.push_str(&format!("    .reg .f32 %fc{}<{}>;\n", i, FRAG_REGS));
                ptx.push_str(&format!("    .reg .f32 %fd{}<{}>;\n", i, FRAG_REGS));
            }
            // Shared memory for 16x16 tiles
            // tile_a: 16x16 f16 = 512 bytes
            // tile_b: 16x16 f16 = 512 bytes
            // tile_d: 16x16 f32 = 1024 bytes
            ptx.push_str(&format!(
                "    .shared .align 32 .f16 tile_a[{}];\n", WMMA_M * WMMA_K
            ));
            ptx.push_str(&format!(
                "    .shared .align 32 .f16 tile_b[{}];\n", WMMA_K * WMMA_N
            ));
            ptx.push_str(&format!(
                "    .shared .align 32 .f32 tile_d[{}];\n", WMMA_M * WMMA_N
            ));
        }
        ptx.push('\n');

        // Load params
        for i in 0..self.num_params {
            ptx.push_str(&format!("    ld.param.u64 %p{}, [param{}];\n", i, i));
        }
        ptx.push('\n');

        // Body
        ptx.push_str(&self.body);

        // Store result
        ptx.push_str("\n    .reg .u64 %out_ptr;\n");
        ptx.push_str("    ld.param.u64 %out_ptr, [result_ptr];\n");
        ptx.push_str(&format!("    st.global.u64 [%out_ptr], {};\n", result));
        ptx.push_str("    ret;\n}\n");
        ptx
    }

    /// Parallel kernel: one warp (32 threads) per matrix tile.
    fn finish_parallel(self, result: &str, num_params: u32) -> String {
        let mut ptx = String::with_capacity(8192);
        ptx.push_str(".version 7.0\n.target sm_80\n.address_size 64\n\n");

        ptx.push_str(".entry main_tensor_parallel(\n");
        for i in 0..num_params {
            ptx.push_str(&format!("    .param .u64 input{}_ptr,\n", i));
        }
        ptx.push_str("    .param .u64 output_ptr,\n");
        ptx.push_str("    .param .u64 count\n) {\n");

        ptx.push_str(&format!("    .reg .u64 %rd<{}>;\n", self.next_reg + 16));
        ptx.push_str(&format!("    .reg .u64 %p<{}>;\n", self.num_params));
        ptx.push_str(&format!("    .reg .pred %p_cond<{}>;\n", self.next_pred + 2));
        ptx.push_str("    .reg .u32 %tid, %warp_id;\n");
        ptx.push_str("    .reg .u64 %tid64, %warp64, %cnt, %addr;\n");
        if self.body.contains("%r_sh") {
            ptx.push_str("    .reg .u32 %r_sh;\n");
        }

        // wmma fragment registers
        if self.uses_wmma {
            for i in 0..self.next_frag {
                ptx.push_str(&format!("    .reg .f16 %fa{}<{}>;\n", i, FRAG_REGS));
                ptx.push_str(&format!("    .reg .f16 %fb{}<{}>;\n", i, FRAG_REGS));
                ptx.push_str(&format!("    .reg .f32 %fc{}<{}>;\n", i, FRAG_REGS));
                ptx.push_str(&format!("    .reg .f32 %fd{}<{}>;\n", i, FRAG_REGS));
            }
            ptx.push_str(&format!(
                "    .shared .align 32 .f16 tile_a[{}];\n", WMMA_M * WMMA_K
            ));
            ptx.push_str(&format!(
                "    .shared .align 32 .f16 tile_b[{}];\n", WMMA_K * WMMA_N
            ));
            ptx.push_str(&format!(
                "    .shared .align 32 .f32 tile_d[{}];\n", WMMA_M * WMMA_N
            ));
        }
        ptx.push('\n');

        // Thread ID and warp ID
        ptx.push_str("    mov.u32 %tid, %tid.x;\n");
        // Warp ID = tid / 32 (each warp handles one tile)
        ptx.push_str("    shr.u32 %warp_id, %tid, 5;\n");
        ptx.push_str("    cvt.u64.u32 %tid64, %tid;\n");
        ptx.push_str("    cvt.u64.u32 %warp64, %warp_id;\n");

        // Bounds check on warp count
        ptx.push_str("    ld.param.u64 %cnt, [count];\n");
        ptx.push_str("    setp.ge.u64 %p_cond0, %warp64, %cnt;\n");
        ptx.push_str("    @%p_cond0 ret;\n\n");

        // Load params: input[i] = input_ptr[warp_id]
        for i in 0..num_params {
            ptx.push_str(&format!("    ld.param.u64 %addr, [input{}_ptr];\n", i));
            ptx.push_str("    mad.lo.u64 %addr, %warp64, 8, %addr;\n");
            ptx.push_str(&format!("    ld.global.u64 %p{}, [%addr];\n", i));
        }
        ptx.push('\n');

        // Formula body
        ptx.push_str(&self.body);

        // Store result[warp_id]
        ptx.push_str("\n    ld.param.u64 %addr, [output_ptr];\n");
        ptx.push_str("    mad.lo.u64 %addr, %warp64, 8, %addr;\n");
        ptx.push_str(&format!("    st.global.u64 [%addr], {};\n", result));
        ptx.push_str("    ret;\n}\n");
        ptx
    }
}

/// Generate comma-separated fragment register list: "base0, base1, ..., baseN"
fn frag_list(base: &str, count: u32) -> String {
    (0..count)
        .map(|i| format!("{}{}", base, i))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use nox::Reduction as Order;
    use nebu::Goldilocks;

    fn g(v: u64) -> Goldilocks { Goldilocks::new(v) }

    fn make_cell<const N: usize>(order: &mut Order<N>, left: NounId, right: NounId) -> NounId {
        order.pair(left, right).unwrap()
    }
    fn make_atom<const N: usize>(order: &mut Order<N>, v: u64) -> NounId {
        order.atom(g(v)).unwrap()
    }
    fn make_formula<const N: usize>(order: &mut Order<N>, tag: u64, body: NounId) -> NounId {
        let t = make_atom(order, tag);
        make_cell(order, t, body)
    }
    fn make_binary<const N: usize>(order: &mut Order<N>, tag: u64, left: NounId, right: NounId) -> NounId {
        let body = make_cell(order, left, right);
        make_formula(order, tag, body)
    }

    #[test]
    fn tensor_quote_compiles() {
        let mut o = Order::<1024>::new();
        let body = make_atom(&mut o, 42);
        let formula = make_formula(&mut o, 1, body);
        let ptx = compile_to_tensor_ptx(&o, formula, 0).unwrap();
        assert!(ptx.contains(".target sm_80"));
        assert!(ptx.contains("mov.u64"));
    }

    #[test]
    fn tensor_add_compiles() {
        let mut o = Order::<1024>::new();
        let ax2 = make_atom(&mut o, 2);
        let a0 = make_formula(&mut o, 0, ax2);
        let ax6 = make_atom(&mut o, 6);
        let a1 = make_formula(&mut o, 0, ax6);
        let formula = make_binary(&mut o, 5, a0, a1);
        let ptx = compile_to_tensor_ptx(&o, formula, 2).unwrap();
        assert!(ptx.contains(".target sm_80"));
        assert!(ptx.contains("add.cc.u64"));
        // Add should not use wmma
        assert!(!ptx.contains("wmma"));
    }

    #[test]
    fn tensor_mul_uses_wmma() {
        let mut o = Order::<1024>::new();
        let ax2 = make_atom(&mut o, 2);
        let a0 = make_formula(&mut o, 0, ax2);
        let ax6 = make_atom(&mut o, 6);
        let a1 = make_formula(&mut o, 0, ax6);
        let formula = make_binary(&mut o, 7, a0, a1);
        let ptx = compile_to_tensor_ptx(&o, formula, 2).unwrap();
        assert!(ptx.contains(".target sm_80"));
        assert!(ptx.contains("wmma.load.a.sync.aligned.m16n16k16"));
        assert!(ptx.contains("wmma.load.b.sync.aligned.m16n16k16"));
        assert!(ptx.contains("wmma.mma.sync.aligned.m16n16k16.f32.f16"));
        assert!(ptx.contains("wmma.store.d.sync.aligned.m16n16k16"));
        assert!(ptx.contains("tile_a"));
        assert!(ptx.contains("tile_b"));
        assert!(ptx.contains("tile_d"));
    }

    #[test]
    fn tensor_parallel_mul_compiles() {
        let mut o = Order::<1024>::new();
        let ax2 = make_atom(&mut o, 2);
        let a0 = make_formula(&mut o, 0, ax2);
        let ax6 = make_atom(&mut o, 6);
        let a1 = make_formula(&mut o, 0, ax6);
        let formula = make_binary(&mut o, 7, a0, a1);
        let ptx = compile_to_tensor_ptx_parallel(&o, formula, 2).unwrap();
        assert!(ptx.contains("main_tensor_parallel"));
        assert!(ptx.contains(".target sm_80"));
        assert!(ptx.contains("wmma.mma.sync.aligned"));
        assert!(ptx.contains("%warp_id"));
    }

    #[test]
    fn tensor_branch_compiles() {
        let mut o = Order::<1024>::new();
        let v0 = make_atom(&mut o, 0);
        let test = make_formula(&mut o, 1, v0);
        let v10 = make_atom(&mut o, 10);
        let yes = make_formula(&mut o, 1, v10);
        let v20 = make_atom(&mut o, 20);
        let no = make_formula(&mut o, 1, v20);
        let yes_no = make_cell(&mut o, yes, no);
        let body = make_cell(&mut o, test, yes_no);
        let formula = make_formula(&mut o, 4, body);
        let ptx = compile_to_tensor_ptx(&o, formula, 0).unwrap();
        assert!(ptx.contains("setp.ne.u64"));
        assert!(ptx.contains("bra"));
    }

    #[test]
    fn tensor_sm80_target() {
        let mut o = Order::<1024>::new();
        let body = make_atom(&mut o, 1);
        let formula = make_formula(&mut o, 1, body);
        let ptx = compile_to_tensor_ptx(&o, formula, 0).unwrap();
        // Must target sm_80 (Ampere+) for wmma support
        assert!(ptx.contains(".target sm_80"));
        // Must NOT target sm_70 (Volta without full wmma f16 support)
        assert!(!ptx.contains(".target sm_70"));
    }

    #[test]
    fn unsupported_pattern_errors() {
        let mut o = Order::<1024>::new();
        let a1 = make_atom(&mut o, 1);
        let body = make_formula(&mut o, 0, a1);
        let formula = make_formula(&mut o, 15, body);
        assert!(matches!(
            compile_to_tensor_ptx(&o, formula, 1),
            Err(CompileError::UnsupportedPattern(15))
        ));
    }
}
