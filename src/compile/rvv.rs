//! RISC-V Vector Extension (RVV 1.0) assembly text emitter for nox formulas
//!
//! Extends RV64 with vector registers v0-v31 and vector instructions.
//! Two modes:
//!   - Single: scalar RV64 evaluation (fallback for non-vectorizable)
//!   - Parallel: vectorized — process VLEN elements per iteration
//!
//! Key RVV concepts:
//!   - `vsetvli rd, rs, vtypei` sets vector length dynamically
//!   - LMUL (register grouping) for wider vectors: m1, m2, m4, m8
//!   - Predicated (masked) operations via v0 mask register
//!   - Element widths: e8, e16, e32, e64
//!
//! All values are u64 in Goldilocks field (p = 2^64 - 2^32 + 1).
//! Output: RV64GV assembly text with .text/.globl directives.

use nox::{Reduction as Order, Order as NounId};
use super::{CompileError, formula_parts, body_pair, body_triple, atom_u64, axis_to_param,
            detect_loop_setup, detect_back_edge};

const P: u64 = 0xFFFF_FFFF_0000_0001;

/// Compile to RVV assembly (single scalar evaluation).
pub fn compile_to_rvv<const N: usize>(
    order: &Order<N>,
    formula: NounId,
    num_params: u32,
) -> Result<String, CompileError> {
    let mut e = RvvEmitter::new(num_params);
    e.emit_formula(order, formula)?;
    let result = e.pop_reg();
    Ok(e.finish_scalar(&result))
}

/// Compile to RVV assembly (parallel: vector operations on arrays).
pub fn compile_to_rvv_parallel<const N: usize>(
    order: &Order<N>,
    formula: NounId,
    num_params: u32,
) -> Result<String, CompileError> {
    let mut e = RvvEmitter::new(num_params);
    e.parallel = true;
    e.emit_formula(order, formula)?;
    let result = e.pop_reg();
    Ok(e.finish_parallel(&result, num_params))
}

struct RvvEmitter {
    body: String,
    num_params: u32,
    next_reg: u32,
    next_label: u32,
    reg_stack: Vec<String>,
    subject: Vec<String>,
    loop_state: Option<RvvLoopState>,
    parallel: bool,
}

#[derive(Clone)]
struct RvvLoopState {
    carried: Vec<String>,
    formula_reg: String,
    header_label: String,
}

impl RvvEmitter {
    fn new(num_params: u32) -> Self {
        let subject: Vec<String> = (0..num_params).rev()
            .map(|i| format!("a{}", i))
            .collect();
        Self {
            body: String::with_capacity(4096),
            num_params,
            next_reg: 0,
            next_label: 0,
            reg_stack: Vec::new(),
            subject,
            loop_state: None,
            parallel: false,
        }
    }

    fn alloc_scalar(&mut self) -> String {
        let r = format!("t{}", self.next_reg);
        self.next_reg += 1;
        r
    }

    fn alloc_vector(&mut self) -> String {
        let r = format!("v{}", self.next_reg);
        self.next_reg += 1;
        r
    }

    fn alloc_label(&mut self) -> String {
        let l = format!(".L{}", self.next_label);
        self.next_label += 1;
        l
    }

    fn push_reg(&mut self) -> String {
        let r = if self.parallel { self.alloc_vector() } else { self.alloc_scalar() };
        self.reg_stack.push(r.clone());
        r
    }

    fn pop_reg(&mut self) -> String {
        self.reg_stack.pop().unwrap_or_else(|| {
            if self.parallel { "v0".to_string() } else { "t0".to_string() }
        })
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
        if self.parallel {
            // Load vector from array pointer
            self.emit(&format!("vle64.v {}, ({})", dst, src));
        } else {
            self.emit(&format!("mv {}, {}", dst, src));
        }
        Ok(())
    }

    fn emit_quote<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let val = atom_u64(order, body)?;
        let dst = self.push_reg();
        if self.parallel {
            // Load immediate into scalar, then splat to vector
            let tmp = self.alloc_scalar();
            self.emit_load_imm64(&tmp, val);
            self.emit(&format!("vmv.v.x {}, {}", dst, tmp));
        } else {
            self.emit_load_imm64(&dst, val);
        }
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
        let formula_reg = if self.parallel { self.alloc_vector() } else { self.alloc_scalar() };
        if self.parallel {
            let tmp = self.alloc_scalar();
            self.emit(&format!("li {}, 0", tmp));
            self.emit(&format!("vmv.v.x {}, {}", formula_reg, tmp));
        } else {
            self.emit(&format!("li {}, 0", formula_reg));
        }

        let mut carried = Vec::new();
        for &init in inits.iter() {
            self.emit_formula(order, init)?;
            let val = self.pop_reg();
            let cr = if self.parallel { self.alloc_vector() } else { self.alloc_scalar() };
            if self.parallel {
                self.emit(&format!("vmv.v.v {}, {}", cr, val));
            } else {
                self.emit(&format!("mv {}, {}", cr, val));
            }
            carried.push(cr);
        }

        let saved = self.subject.clone();
        for cr in carried.iter() {
            self.subject.insert(0, cr.clone());
        }
        self.subject.insert(0, formula_reg.clone());

        let header = self.alloc_label();
        let prev = self.loop_state.take();
        self.loop_state = Some(RvvLoopState {
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
                if self.parallel {
                    self.emit(&format!("vmv.v.v {}, {}", cr, new_vals[i]));
                } else {
                    self.emit(&format!("mv {}, {}", cr, new_vals[i]));
                }
            }
        }

        self.emit(&format!("j {}", ls.header_label));
        let _ = self.push_reg(); // dummy
        Ok(())
    }

    fn emit_branch<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (test, yes, no) = body_triple(order, body)?;
        self.emit_formula(order, test)?;
        let test_r = self.pop_reg();
        let lbl_yes = self.alloc_label();
        let lbl_end = self.alloc_label();

        if self.parallel {
            // Vector branch: use mask register v0 for predication
            // nox: 0=yes, nonzero=no. Set mask where test==0 (yes lanes).
            let mask_tmp = self.alloc_vector();
            let zero_v = self.alloc_vector();
            let tmp_s = self.alloc_scalar();
            self.emit(&format!("li {}, 0", tmp_s));
            self.emit(&format!("vmv.v.x {}, {}", zero_v, tmp_s));
            self.emit(&format!("vmseq.vv {}, {}, {}", mask_tmp, test_r, zero_v));

            // Compute both paths, merge via vmerge
            self.emit_formula(order, yes)?;
            let yes_r = self.pop_reg();
            self.emit_formula(order, no)?;
            let no_r = self.pop_reg();
            let dst = self.push_reg();

            // Start with no-path, merge yes-path where mask is set
            self.emit(&format!("vmv.v.v {}, {}", dst, no_r));
            // Move mask to v0 for masked merge
            self.emit(&format!("vmv.v.v v0, {}", mask_tmp));
            self.emit(&format!("vmerge.vvm {}, {}, {}, v0", dst, dst, yes_r));
        } else {
            // Scalar branch: nox 0=yes
            self.emit(&format!("beqz {}, {}", test_r, lbl_yes));

            // no path
            self.emit_formula(order, no)?;
            let no_r = self.pop_reg();
            let dst = self.push_reg();
            self.emit(&format!("mv {}, {}", dst, no_r));
            self.emit(&format!("j {}", lbl_end));
            self.pop_reg();

            // yes path
            self.emit_label(&lbl_yes);
            self.emit_formula(order, yes)?;
            let yes_r = self.pop_reg();
            let dst2 = self.push_reg();
            self.emit(&format!("mv {}, {}", dst2, yes_r));

            self.emit_label(&lbl_end);
        }
        Ok(())
    }

    fn emit_add<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();

        if self.parallel {
            self.emit(&format!("vadd.vv {}, {}, {}", dst, ra, rb));
            self.emit_vector_goldilocks_reduce_add(&dst, &ra);
        } else {
            self.emit(&format!("add {}, {}, {}", dst, ra, rb));
            self.emit_scalar_goldilocks_reduce_add(&dst, &ra);
        }
        Ok(())
    }

    fn emit_sub<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();

        if self.parallel {
            // Vector sub with underflow handling
            let mask = self.alloc_vector();
            let p_vec = self.alloc_vector();
            let tmp_s = self.alloc_scalar();

            // mask = lanes where ra < rb (underflow)
            self.emit(&format!("vmsltu.vv {}, {}, {}", mask, ra, rb));
            // Normal: dst = ra - rb
            self.emit(&format!("vsub.vv {}, {}, {}", dst, ra, rb));
            // Underflow lanes: dst = P - rb + ra = P + dst (since dst = ra-rb wraps)
            self.emit_load_imm64(&tmp_s, P);
            self.emit(&format!("vmv.v.x {}, {}", p_vec, tmp_s));
            // For underflow lanes, add P
            self.emit(&format!("vmv.v.v v0, {}", mask));
            self.emit(&format!("vadd.vv {}, {}, {}, v0.t", dst, dst, p_vec));
        } else {
            let lbl_ok = self.alloc_label();
            let lbl_end = self.alloc_label();

            self.emit(&format!("bgeu {}, {}, {}", ra, rb, lbl_ok));
            // underflow: P - rb + ra
            let tmp = self.alloc_scalar();
            self.emit_load_imm64(&tmp, P);
            self.emit(&format!("sub {}, {}, {}", dst, tmp, rb));
            self.emit(&format!("add {}, {}, {}", dst, dst, ra));
            self.emit(&format!("j {}", lbl_end));

            self.emit_label(&lbl_ok);
            self.emit(&format!("sub {}, {}, {}", dst, ra, rb));
            self.emit_label(&lbl_end);
        }
        Ok(())
    }

    fn emit_mul<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();

        if self.parallel {
            // Vector multiply: vmul.vv gives low 64 bits, vmulhu.vv gives high 64 bits
            let hi = self.alloc_vector();
            let tmp = self.alloc_vector();

            self.emit(&format!("vmul.vv {}, {}, {}", dst, ra, rb));
            self.emit(&format!("vmulhu.vv {}, {}, {}", hi, ra, rb));

            // Goldilocks reduction: result = lo + hi*(2^32-1) mod P
            // tmp = hi << 32
            self.emit(&format!("vsll.vi {}, {}, 32", tmp, hi));
            // dst = lo + (hi << 32)
            let saved_lo = self.alloc_vector();
            self.emit(&format!("vmv.v.v {}, {}", saved_lo, dst));
            self.emit(&format!("vadd.vv {}, {}, {}", dst, dst, tmp));
            // dst -= hi
            self.emit(&format!("vsub.vv {}, {}, {}", dst, dst, hi));
            // Final reduce mod P
            self.emit_vector_reduce_mod_p(&dst);
        } else {
            let hi = self.alloc_scalar();
            let tmp = self.alloc_scalar();

            self.emit(&format!("mul {}, {}, {}", dst, ra, rb));
            self.emit(&format!("mulhu {}, {}, {}", hi, ra, rb));

            // Goldilocks reduction
            self.emit(&format!("slli {}, {}, 32", tmp, hi));
            self.emit(&format!("add {}, {}, {}", dst, dst, tmp));
            self.emit(&format!("sub {}, {}, {}", dst, dst, hi));

            // if dst >= P: dst -= P
            self.emit_scalar_reduce_mod_p(&dst);
        }
        Ok(())
    }

    fn emit_eq<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();

        if self.parallel {
            // nox eq: 0 if equal, 1 if not
            let mask = self.alloc_vector();
            let tmp_s = self.alloc_scalar();
            self.emit(&format!("vmsne.vv {}, {}, {}", mask, ra, rb));
            // Convert mask to 0/1 vector
            self.emit(&format!("li {}, 0", tmp_s));
            self.emit(&format!("vmv.v.x {}, {}", dst, tmp_s));
            self.emit(&format!("vmv.v.v v0, {}", mask));
            self.emit(&format!("li {}, 1", tmp_s));
            self.emit(&format!("vmerge.vxm {}, {}, {}, v0", dst, dst, tmp_s));
        } else {
            // dst = (ra != rb) ? 1 : 0
            self.emit(&format!("sub {}, {}, {}", dst, ra, rb));
            self.emit(&format!("snez {}, {}", dst, dst));
        }
        Ok(())
    }

    fn emit_lt<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();

        if self.parallel {
            // nox lt: 0 if a<b, 1 if a>=b
            let mask = self.alloc_vector();
            let tmp_s = self.alloc_scalar();
            self.emit(&format!("vmsgeu.vv {}, {}, {}", mask, ra, rb));
            self.emit(&format!("li {}, 0", tmp_s));
            self.emit(&format!("vmv.v.x {}, {}", dst, tmp_s));
            self.emit(&format!("vmv.v.v v0, {}", mask));
            self.emit(&format!("li {}, 1", tmp_s));
            self.emit(&format!("vmerge.vxm {}, {}, {}, v0", dst, dst, tmp_s));
        } else {
            // nox lt: 0 if a<b, 1 if a>=b
            self.emit(&format!("sltu {}, {}, {}", dst, ra, rb));
            self.emit(&format!("xori {}, {}, 1", dst, dst));
        }
        Ok(())
    }

    fn emit_xor<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();

        if self.parallel {
            self.emit(&format!("vxor.vv {}, {}, {}", dst, ra, rb));
        } else {
            self.emit(&format!("xor {}, {}, {}", dst, ra, rb));
        }
        Ok(())
    }

    fn emit_and<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();

        if self.parallel {
            self.emit(&format!("vand.vv {}, {}, {}", dst, ra, rb));
        } else {
            self.emit(&format!("and {}, {}, {}", dst, ra, rb));
        }
        Ok(())
    }

    fn emit_not<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        self.emit_formula(order, body)?;
        let ra = self.pop_reg();
        let dst = self.push_reg();

        if self.parallel {
            let mask_v = self.alloc_vector();
            let tmp_s = self.alloc_scalar();
            self.emit(&format!("vnot.v {}, {}", dst, ra));
            self.emit_load_imm64(&tmp_s, 0xFFFF_FFFF);
            self.emit(&format!("vmv.v.x {}, {}", mask_v, tmp_s));
            self.emit(&format!("vand.vv {}, {}, {}", dst, dst, mask_v));
        } else {
            let tmp = self.alloc_scalar();
            self.emit(&format!("not {}, {}", dst, ra));
            self.emit_load_imm64(&tmp, 0xFFFF_FFFF);
            self.emit(&format!("and {}, {}, {}", dst, dst, tmp));
        }
        Ok(())
    }

    fn emit_shl<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();

        if self.parallel {
            let mask_v = self.alloc_vector();
            let tmp_s = self.alloc_scalar();
            self.emit(&format!("vsll.vv {}, {}, {}", dst, ra, rb));
            self.emit_load_imm64(&tmp_s, 0xFFFF_FFFF);
            self.emit(&format!("vmv.v.x {}, {}", mask_v, tmp_s));
            self.emit(&format!("vand.vv {}, {}, {}", dst, dst, mask_v));
        } else {
            let tmp = self.alloc_scalar();
            self.emit(&format!("sll {}, {}, {}", dst, ra, rb));
            self.emit_load_imm64(&tmp, 0xFFFF_FFFF);
            self.emit(&format!("and {}, {}, {}", dst, dst, tmp));
        }
        Ok(())
    }

    // ── Goldilocks reduction helpers ─────────────────────────────

    /// Scalar: if sum overflowed (sum < original_a), add 0xFFFFFFFF.
    /// Then if sum >= P, subtract P.
    fn emit_scalar_goldilocks_reduce_add(&mut self, dst: &str, original_a: &str) {
        let lbl_no_overflow = self.alloc_label();
        let tmp = self.alloc_scalar();

        // if dst >= original_a: no overflow
        self.emit(&format!("bgeu {}, {}, {}", dst, original_a, lbl_no_overflow));
        self.emit_load_imm64(&tmp, 0xFFFF_FFFF);
        self.emit(&format!("add {}, {}, {}", dst, dst, tmp));
        self.emit_label(&lbl_no_overflow);

        // if dst >= P: dst -= P
        self.emit_scalar_reduce_mod_p(dst);
    }

    /// Scalar: if dst >= P, subtract P.
    fn emit_scalar_reduce_mod_p(&mut self, dst: &str) {
        let lbl_skip = self.alloc_label();
        let tmp = self.alloc_scalar();
        self.emit_load_imm64(&tmp, P);
        self.emit(&format!("bltu {}, {}, {}", dst, tmp, lbl_skip));
        self.emit(&format!("sub {}, {}, {}", dst, dst, tmp));
        self.emit_label(&lbl_skip);
    }

    /// Vector: overflow handling for add, then reduce mod P.
    fn emit_vector_goldilocks_reduce_add(&mut self, dst: &str, original_a: &str) {
        let mask = self.alloc_vector();
        let corr = self.alloc_vector();
        let tmp_s = self.alloc_scalar();

        // Detect overflow: dst < original_a
        self.emit(&format!("vmsltu.vv {}, {}, {}", mask, dst, original_a));
        // Add correction 0xFFFFFFFF to overflow lanes
        self.emit_load_imm64(&tmp_s, 0xFFFF_FFFF);
        self.emit(&format!("vmv.v.x {}, {}", corr, tmp_s));
        self.emit(&format!("vmv.v.v v0, {}", mask));
        self.emit(&format!("vadd.vv {}, {}, {}, v0.t", dst, dst, corr));

        // Reduce mod P
        self.emit_vector_reduce_mod_p(dst);
    }

    /// Vector: if element >= P, subtract P.
    fn emit_vector_reduce_mod_p(&mut self, dst: &str) {
        let mask = self.alloc_vector();
        let p_vec = self.alloc_vector();
        let tmp_s = self.alloc_scalar();

        self.emit_load_imm64(&tmp_s, P);
        self.emit(&format!("vmv.v.x {}, {}", p_vec, tmp_s));
        // mask = lanes where dst >= P
        self.emit(&format!("vmsgeu.vv {}, {}, {}", mask, dst, p_vec));
        self.emit(&format!("vmv.v.v v0, {}", mask));
        self.emit(&format!("vsub.vv {}, {}, {}, v0.t", dst, dst, p_vec));
    }

    /// Emit scalar load of 64-bit immediate.
    fn emit_load_imm64(&mut self, rd: &str, val: u64) {
        if val == 0 {
            self.emit(&format!("li {}, 0", rd));
        } else if val < 2048 {
            self.emit(&format!("li {}, {}", rd, val));
        } else {
            // Use li pseudo-instruction — assembler handles expansion
            self.emit(&format!("li {}, 0x{:x}", rd, val));
        }
    }

    /// Scalar function: args in a0-a7, result in a0.
    fn finish_scalar(self, result: &str) -> String {
        let mut asm = String::with_capacity(4096);
        asm.push_str("# RV64GV assembly — scalar evaluation\n");
        asm.push_str("# Goldilocks field: p = 2^64 - 2^32 + 1\n\n");
        asm.push_str("    .text\n");
        asm.push_str("    .globl nox_eval\n");
        asm.push_str("    .type nox_eval, @function\n");
        asm.push_str("nox_eval:\n");
        asm.push_str("    # args: a0-a7, result in a0\n");

        asm.push_str(&self.body);

        // Move result to a0
        if result != "a0" {
            asm.push_str(&format!("    mv a0, {}\n", result));
        }
        asm.push_str("    ret\n");
        asm.push_str("    .size nox_eval, .-nox_eval\n");
        asm
    }

    /// Parallel function: process arrays with vector instructions.
    /// a0 = count, a1..aN = input array pointers, a(N+1) = output array pointer.
    fn finish_parallel(self, result: &str, num_params: u32) -> String {
        let out_reg = format!("s{}", num_params + 1);

        let mut asm = String::with_capacity(8192);
        asm.push_str("# RV64GV assembly — vectorized parallel evaluation\n");
        asm.push_str("# Goldilocks field: p = 2^64 - 2^32 + 1\n");
        asm.push_str("# Process VLEN elements per iteration via RVV 1.0\n\n");
        asm.push_str("    .text\n");
        asm.push_str("    .globl nox_eval_parallel\n");
        asm.push_str("    .type nox_eval_parallel, @function\n");
        asm.push_str("nox_eval_parallel:\n");
        asm.push_str(&format!("    # a0 = count, a1..a{} = input ptrs, a{} = output ptr\n",
            num_params, num_params + 1));

        // Save count and pointers into callee-saved registers
        asm.push_str("    mv s0, a0              # s0 = remaining count\n");
        for i in 0..num_params {
            asm.push_str(&format!("    mv s{}, a{}              # s{} = input{} ptr\n",
                i + 1, i + 1, i + 1, i));
        }
        asm.push_str(&format!("    mv {}, a{}              # {} = output ptr\n",
            out_reg, num_params + 1, out_reg));

        // Strip-mining loop: process up to VLEN elements per iteration
        asm.push_str("\n.Lloop:\n");
        asm.push_str("    beqz s0, .Ldone\n");
        // vsetvli sets vl = min(s0, VLMAX) for e64/m1, returns vl in t6
        asm.push_str("    vsetvli t6, s0, e64, m1, ta, ma\n");

        // Load input vectors from array pointers
        for i in 0..num_params {
            asm.push_str(&format!("    vle64.v v{}, (s{})\n", 16 + i, i + 1));
        }
        asm.push('\n');

        // Formula body (operates on vector registers)
        asm.push_str(&self.body);

        // Store result vector to output array
        asm.push_str(&format!("\n    vse64.v {}, ({})\n", result, out_reg));

        // Advance: subtract vl from element count, advance pointers by vl*8 bytes
        // t6 still holds vl from vsetvli above
        asm.push_str("    sub s0, s0, t6         # remaining count -= vl\n");
        asm.push_str("    slli t6, t6, 3         # t6 = vl * 8 (byte stride)\n");
        for i in 0..num_params {
            asm.push_str(&format!("    add s{}, s{}, t6         # advance input{} ptr\n",
                i + 1, i + 1, i));
        }
        asm.push_str(&format!("    add {}, {}, t6         # advance output ptr\n",
            out_reg, out_reg));
        asm.push_str("    j .Lloop\n");

        asm.push_str("\n.Ldone:\n");
        asm.push_str("    ret\n");
        asm.push_str("    .size nox_eval_parallel, .-nox_eval_parallel\n");
        asm
    }
}
