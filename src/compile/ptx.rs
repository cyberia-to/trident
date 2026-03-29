//! CUDA PTX text emitter for nox formulas
//!
//! Compiles nox formulas to NVIDIA PTX assembly.
//! Two modes:
//!   - Single: .entry main(params...) → result in %rd0
//!   - Parallel: .entry main(inputs, outputs, count) → batch evaluate
//!
//! PTX has mul.hi.u64 for upper 64 bits — Goldilocks mul is clean.
//! All values are u64 in Goldilocks field (p = 2^64 - 2^32 + 1).






use nox::noun::{Order, NounId};
use super::{CompileError, formula_parts, body_pair, body_triple, atom_u64, axis_to_param,
            detect_loop_setup, detect_back_edge};

const P: u64 = 0xFFFF_FFFF_0000_0001;

/// Compile to PTX (single evaluation).
pub fn compile_to_ptx<const N: usize>(
    order: &Order<N>,
    formula: NounId,
    num_params: u32,
) -> Result<String, CompileError> {
    let mut e = PtxEmitter::new(num_params);
    e.emit_formula(order, formula)?;
    let result = e.pop_reg();
    Ok(e.finish(&result))
}

/// Compile to PTX (parallel: one thread per input).
pub fn compile_to_ptx_parallel<const N: usize>(
    order: &Order<N>,
    formula: NounId,
    num_params: u32,
) -> Result<String, CompileError> {
    let mut e = PtxEmitter::new(num_params);
    e.emit_formula(order, formula)?;
    let result = e.pop_reg();
    Ok(e.finish_parallel(&result, num_params))
}

struct PtxEmitter {
    body: String,
    num_params: u32,
    next_reg: u32,
    next_pred: u32,
    next_label: u32,
    reg_stack: Vec<String>,
    subject: Vec<String>,
    loop_state: Option<PtxLoopState>,
}

#[derive(Clone)]
struct PtxLoopState {
    carried: Vec<String>,
    formula_reg: String,
    header_label: String,
}

impl PtxEmitter {
    fn new(num_params: u32) -> Self {
        let subject: Vec<String> = (0..num_params).rev()
            .map(|i| format!("%p{}", i))
            .collect();
        Self {
            body: String::with_capacity(2048),
            num_params,
            next_reg: 0,
            next_pred: 0,
            next_label: 0,
            reg_stack: Vec::new(),
            subject,
            loop_state: None,
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

    fn push_reg(&mut self) -> String {
        let r = self.alloc_reg();
        self.reg_stack.push(r.clone());
        r
    }

    fn pop_reg(&mut self) -> String {
        self.reg_stack.pop().unwrap_or_else(|| format!("%rd0"))
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
        self.loop_state = Some(PtxLoopState {
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
        // Compile new values into temps first, then move to carried regs
        // (avoids clobbering a carried reg that's still needed)
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
        // Detect carry via comparison: if dst < ra then overflow
        self.emit(&format!("setp.lt.u64 {}, {}, {};", carry, dst, ra));
        self.emit(&format!("mov.u64 {}, 4294967295;", tmp)); // 0xFFFFFFFF
        self.emit(&format!("@{} add.u64 {}, {}, {};", carry, dst, dst, tmp));
        // if dst >= P: dst -= P
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

    fn emit_mul<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        let hi = self.alloc_reg();
        let tmp = self.alloc_reg();
        let carry = self.alloc_pred();
        let borrow = self.alloc_pred();

        // PTX has mul.hi.u64 — clean 128-bit multiply!
        self.emit(&format!("mul.lo.u64 {}, {}, {};", dst, ra, rb));
        self.emit(&format!("mul.hi.u64 {}, {}, {};", hi, ra, rb));

        // Reduce: dst = lo + hi*(2^32-1) mod P
        // tmp = hi << 32
        self.emit(&format!("shl.b64 {}, {}, 32;", tmp, hi));
        // dst += tmp (may carry)
        let saved_lo = self.alloc_reg();
        self.emit(&format!("mov.u64 {}, {};", saved_lo, dst));
        self.emit(&format!("add.u64 {}, {}, {};", dst, dst, tmp));
        self.emit(&format!("setp.lt.u64 {}, {}, {};", carry, dst, saved_lo));
        self.emit(&format!("mov.u64 {}, 4294967295;", tmp));
        self.emit(&format!("@{} add.u64 {}, {}, {};", carry, dst, dst, tmp));
        // dst -= hi
        self.emit(&format!("mov.u64 {}, {};", saved_lo, dst));
        self.emit(&format!("sub.u64 {}, {}, {};", dst, dst, hi));
        self.emit(&format!("setp.gt.u64 {}, {}, {};", borrow, dst, saved_lo));
        self.emit(&format!("@{} sub.u64 {}, {}, {};", borrow, dst, dst, tmp));
        // Final reduce
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
        let tmp = self.alloc_reg();
        // PTX shl needs 32-bit shift amount
        self.emit(&format!("cvt.u32.u64 %r_sh, {};", rb));
        self.emit(&format!("shl.b64 {}, {}, %r_sh;", dst, ra));
        self.emit(&format!("mov.u64 {}, 4294967295;", tmp));
        self.emit(&format!("and.b64 {}, {}, {};", dst, dst, tmp));
        Ok(())
    }

    fn emit_reduce_mod_p(&mut self, dst: &str) {
        let pred = self.alloc_pred();
        let tmp = self.alloc_reg();
        self.emit(&format!("mov.u64 {}, {};", tmp, P));
        self.emit(&format!("setp.ge.u64 {}, {}, {};", pred, dst, tmp));
        self.emit(&format!("@{} sub.u64 {}, {}, {};", pred, dst, dst, tmp));
    }

    /// Single evaluation kernel.
    fn finish(self, result: &str) -> String {
        let mut ptx = String::with_capacity(4096);
        ptx.push_str(".version 7.0\n.target sm_70\n.address_size 64\n\n");

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
        ptx.push('\n');

        // Load params
        for i in 0..self.num_params {
            ptx.push_str(&format!("    ld.param.u64 %p{}, [param{}];\n", i, i));
        }
        ptx.push('\n');

        // Body
        ptx.push_str(&self.body);

        // Store result
        ptx.push_str(&format!("\n    .reg .u64 %out_ptr;\n"));
        ptx.push_str(&format!("    ld.param.u64 %out_ptr, [result_ptr];\n"));
        ptx.push_str(&format!("    st.global.u64 [%out_ptr], {};\n", result));
        ptx.push_str("    ret;\n}\n");
        ptx
    }

    /// Parallel kernel: one thread per input element.
    fn finish_parallel(self, result: &str, num_params: u32) -> String {
        let mut ptx = String::with_capacity(4096);
        ptx.push_str(".version 7.0\n.target sm_70\n.address_size 64\n\n");

        ptx.push_str(".entry main_parallel(\n");
        for i in 0..num_params {
            ptx.push_str(&format!("    .param .u64 input{}_ptr,\n", i));
        }
        ptx.push_str("    .param .u64 output_ptr,\n");
        ptx.push_str("    .param .u64 count\n) {\n");

        ptx.push_str(&format!("    .reg .u64 %rd<{}>;\n", self.next_reg + 16));
        ptx.push_str(&format!("    .reg .u64 %p<{}>;\n", self.num_params));
        ptx.push_str(&format!("    .reg .pred %p_cond<{}>;\n", self.next_pred + 2));
        ptx.push_str("    .reg .u32 %tid;\n");
        ptx.push_str("    .reg .u64 %tid64, %cnt, %addr;\n");
        if self.body.contains("%r_sh") {
            ptx.push_str("    .reg .u32 %r_sh;\n");
        }
        ptx.push('\n');

        // Thread ID + bounds check
        ptx.push_str("    mov.u32 %tid, %tid.x;\n");
        ptx.push_str("    cvt.u64.u32 %tid64, %tid;\n");
        ptx.push_str("    ld.param.u64 %cnt, [count];\n");
        ptx.push_str("    setp.ge.u64 %p_cond0, %tid64, %cnt;\n");
        ptx.push_str("    @%p_cond0 ret;\n\n");

        // Load params: input[i] = input_ptr[tid]
        for i in 0..num_params {
            ptx.push_str(&format!("    ld.param.u64 %addr, [input{}_ptr];\n", i));
            ptx.push_str(&format!("    mad.lo.u64 %addr, %tid64, 8, %addr;\n"));
            ptx.push_str(&format!("    ld.global.u64 %p{}, [%addr];\n", i));
        }
        ptx.push('\n');

        // Formula body
        ptx.push_str(&self.body);

        // Store result[tid]
        ptx.push_str(&format!("\n    ld.param.u64 %addr, [output_ptr];\n"));
        ptx.push_str(&format!("    mad.lo.u64 %addr, %tid64, 8, %addr;\n"));
        ptx.push_str(&format!("    st.global.u64 [%addr], {};\n", result));
        ptx.push_str("    ret;\n}\n");
        ptx
    }
}
