//! UPMEM PIM (Processing-in-Memory) C emitter for nox formulas
//!
//! Compiles nox formulas to DPU C source code for UPMEM PIM hardware.
//! Each DRAM chip has multiple DPUs (DRAM Processing Units), 32-bit RISC
//! processors embedded directly in memory. Each DPU runs up to 24 tasklets
//! (hardware threads).
//!
//! Two modes:
//!   - Single: main() evaluates formula once per tasklet
//!   - Parallel: each tasklet evaluates formula on its slice of MRAM data
//!
//! Output: C source text compiled via `dpu-upmem-dpurte-clang`.
//! All values are u64 in Goldilocks field (p = 2^64 - 2^32 + 1).

use nox::{Reduction as Order, Order as NounId};
use super::{CompileError, formula_parts, body_pair, body_triple, atom_u64, axis_to_param,
            detect_loop_setup, detect_back_edge};

const P: u64 = 0xFFFF_FFFF_0000_0001;

/// Compile to DPU C (single evaluation per tasklet).
pub fn compile_to_upmem<const N: usize>(
    order: &Order<N>,
    formula: NounId,
    num_params: u32,
) -> Result<String, CompileError> {
    let mut e = UpmemEmitter::new(num_params);
    e.emit_formula(order, formula)?;
    let result = e.pop_reg();
    Ok(e.finish(&result))
}

/// Compile to DPU C (parallel: each tasklet processes a chunk of MRAM data).
pub fn compile_to_upmem_parallel<const N: usize>(
    order: &Order<N>,
    formula: NounId,
    num_params: u32,
) -> Result<String, CompileError> {
    let mut e = UpmemEmitter::new(num_params);
    e.emit_formula(order, formula)?;
    let result = e.pop_reg();
    Ok(e.finish_parallel(&result, num_params))
}

struct UpmemEmitter {
    body: String,
    num_params: u32,
    next_var: u32,
    next_label: u32,
    reg_stack: Vec<String>,
    subject: Vec<String>,
    loop_state: Option<UpmemLoopState>,
}

#[derive(Clone)]
struct UpmemLoopState {
    carried: Vec<String>,
    header_label: String,
}

impl UpmemEmitter {
    fn new(num_params: u32) -> Self {
        let subject: Vec<String> = (0..num_params).rev()
            .map(|i| format!("p{}", i))
            .collect();
        Self {
            body: String::with_capacity(2048),
            num_params,
            next_var: 0,
            next_label: 0,
            reg_stack: Vec::new(),
            subject,
            loop_state: None,
        }
    }

    fn alloc_var(&mut self) -> String {
        let v = format!("v{}", self.next_var);
        self.next_var += 1;
        v
    }

    fn alloc_label(&mut self) -> String {
        let l = format!("L{}", self.next_label);
        self.next_label += 1;
        l
    }

    fn push_reg(&mut self) -> String {
        let v = self.alloc_var();
        self.reg_stack.push(v.clone());
        v
    }

    fn pop_reg(&mut self) -> String {
        self.reg_stack.pop().unwrap_or_else(|| "v0".into())
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
        self.emit(&format!("uint64_t {} = {};", dst, src));
        Ok(())
    }

    fn emit_quote<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let val = atom_u64(order, body)?;
        let dst = self.push_reg();
        self.emit(&format!("uint64_t {} = UINT64_C({});", dst, val));
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
        let formula_var = self.alloc_var();
        self.emit(&format!("uint64_t {} = 0;", formula_var));

        let mut carried = Vec::new();
        for &init in inits.iter() {
            self.emit_formula(order, init)?;
            let val = self.pop_reg();
            let cr = self.alloc_var();
            self.emit(&format!("uint64_t {} = {};", cr, val));
            carried.push(cr);
        }

        let saved = self.subject.clone();
        for cr in carried.iter() {
            self.subject.insert(0, cr.clone());
        }
        self.subject.insert(0, formula_var.clone());

        let header = self.alloc_label();
        let prev = self.loop_state.take();
        self.loop_state = Some(UpmemLoopState {
            carried: carried.clone(),
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
                self.emit(&format!("{} = {};", cr, new_vals[i]));
            }
        }

        self.emit(&format!("goto {};", ls.header_label));
        let _ = self.push_reg(); // dummy
        Ok(())
    }

    fn emit_branch<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (test, yes, no) = body_triple(order, body)?;
        self.emit_formula(order, test)?;
        let test_r = self.pop_reg();
        let dst = self.alloc_var();
        let lbl_no = self.alloc_label();
        let lbl_end = self.alloc_label();

        self.emit(&format!("uint64_t {};", dst));
        // nox: 0=yes, nonzero=no
        self.emit(&format!("if ({} != 0) goto {};", test_r, lbl_no));

        // yes path (test==0)
        self.emit_formula(order, yes)?;
        let yes_r = self.pop_reg();
        self.emit(&format!("{} = {};", dst, yes_r));
        self.emit(&format!("goto {};", lbl_end));

        // no path
        self.emit_label(&lbl_no);
        self.emit_formula(order, no)?;
        let no_r = self.pop_reg();
        self.emit(&format!("{} = {};", dst, no_r));

        self.emit_label(&lbl_end);
        self.reg_stack.push(dst);
        Ok(())
    }

    fn emit_add<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();

        // Goldilocks add with carry reduction
        self.emit(&format!("uint64_t {} = {} + {};", dst, ra, rb));
        self.emit(&format!("if ({0} < {1}) {0} += UINT64_C(0xFFFFFFFF);", dst, ra));
        self.emit(&format!("if ({0} >= UINT64_C({1})) {0} -= UINT64_C({1});", dst, P));
        Ok(())
    }

    fn emit_sub<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();

        self.emit(&format!("uint64_t {};", dst));
        self.emit(&format!("if ({} >= {}) {{", ra, rb));
        self.emit(&format!("    {} = {} - {};", dst, ra, rb));
        self.emit("} else {");
        self.emit(&format!("    {} = UINT64_C({}) - {} + {};", dst, P, rb, ra));
        self.emit("}");
        Ok(())
    }

    fn emit_mul<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        let hi = self.alloc_var();

        // 128-bit multiply via __uint128_t (GCC/Clang extension, supported by DPU toolchain)
        self.emit(&format!("__uint128_t wide_{} = (__uint128_t){} * {};", dst, ra, rb));
        self.emit(&format!("uint64_t {} = (uint64_t)wide_{};", dst, dst));
        self.emit(&format!("uint64_t {} = (uint64_t)(wide_{} >> 64);", hi, dst));

        // Reduce: lo + hi*(2^32-1) mod P
        let tmp = self.alloc_var();
        let saved = self.alloc_var();
        self.emit(&format!("uint64_t {} = {} << 32;", tmp, hi));
        self.emit(&format!("uint64_t {} = {};", saved, dst));
        self.emit(&format!("{} += {};", dst, tmp));
        self.emit(&format!("if ({0} < {1}) {0} += UINT64_C(0xFFFFFFFF);", dst, saved));
        self.emit(&format!("{} = {};", saved, dst));
        self.emit(&format!("{} -= {};", dst, hi));
        self.emit(&format!("if ({0} > {1}) {0} -= UINT64_C(0xFFFFFFFF);", dst, saved));
        self.emit(&format!("if ({0} >= UINT64_C({1})) {0} -= UINT64_C({1});", dst, P));
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
        self.emit(&format!("uint64_t {} = ({} != {}) ? 1 : 0;", dst, ra, rb));
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
        self.emit(&format!("uint64_t {} = ({} >= {}) ? 1 : 0;", dst, ra, rb));
        Ok(())
    }

    fn emit_xor<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("uint64_t {} = {} ^ {};", dst, ra, rb));
        Ok(())
    }

    fn emit_and<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("uint64_t {} = {} & {};", dst, ra, rb));
        Ok(())
    }

    fn emit_not<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        self.emit_formula(order, body)?;
        let ra = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("uint64_t {} = ~{} & UINT64_C(0xFFFFFFFF);", dst, ra));
        Ok(())
    }

    fn emit_shl<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("uint64_t {} = ({} << {}) & UINT64_C(0xFFFFFFFF);", dst, ra, rb));
        Ok(())
    }

    /// Single evaluation kernel: one formula eval per tasklet.
    fn finish(self, result: &str) -> String {
        let mut c = String::with_capacity(4096);

        // UPMEM headers
        c.push_str("#include <stdint.h>\n");
        c.push_str("#include <mram.h>\n");
        c.push_str("#include <defs.h>\n");
        c.push_str("#include <perfcounter.h>\n\n");

        // MRAM buffers: inputs and outputs per tasklet
        c.push_str("// MRAM input/output buffers\n");
        for i in 0..self.num_params {
            c.push_str(&format!("__mram_noinit uint64_t input{}[NR_TASKLETS];\n", i));
        }
        c.push_str("__mram_noinit uint64_t output[NR_TASKLETS];\n\n");

        c.push_str("int main() {\n");
        c.push_str("    uint32_t tid = me();\n\n");

        // Load params from MRAM
        for i in 0..self.num_params {
            c.push_str(&format!("    uint64_t p{} = input{}[tid];\n", i, i));
        }
        c.push('\n');

        // Formula body
        c.push_str(&self.body);

        // Store result to MRAM
        c.push_str(&format!("\n    output[tid] = {};\n", result));
        c.push_str("    return 0;\n");
        c.push_str("}\n");
        c
    }

    /// Parallel kernel: each tasklet processes a chunk of MRAM data.
    fn finish_parallel(self, result: &str, num_params: u32) -> String {
        let mut c = String::with_capacity(4096);

        // UPMEM headers
        c.push_str("#include <stdint.h>\n");
        c.push_str("#include <mram.h>\n");
        c.push_str("#include <defs.h>\n");
        c.push_str("#include <perfcounter.h>\n\n");

        // MRAM buffers: large arrays, partitioned across tasklets
        c.push_str("// Element count (set by host before launch)\n");
        c.push_str("__mram_noinit uint32_t nr_elements;\n\n");
        for i in 0..num_params {
            c.push_str(&format!("__mram_noinit uint64_t input{}[1 << 20];\n", i));
        }
        c.push_str("__mram_noinit uint64_t output_buf[1 << 20];\n\n");

        c.push_str("int main() {\n");
        c.push_str("    uint32_t tid = me();\n");
        c.push_str("    uint32_t count = nr_elements;\n");
        c.push_str("    uint32_t chunk = (count + NR_TASKLETS - 1) / NR_TASKLETS;\n");
        c.push_str("    uint32_t start = tid * chunk;\n");
        c.push_str("    uint32_t end = start + chunk;\n");
        c.push_str("    if (end > count) end = count;\n\n");

        c.push_str("    for (uint32_t idx = start; idx < end; idx++) {\n");

        // Load params from MRAM per element
        for i in 0..num_params {
            c.push_str(&format!("        uint64_t p{} = input{}[idx];\n", i, i));
        }
        c.push('\n');

        // Indent body by extra level inside the loop
        for line in self.body.lines() {
            c.push_str("    ");
            c.push_str(line);
            c.push('\n');
        }

        // Store result
        c.push_str(&format!("\n        output_buf[idx] = {};\n", result));
        c.push_str("    }\n");
        c.push_str("    return 0;\n");
        c.push_str("}\n");
        c
    }
}
