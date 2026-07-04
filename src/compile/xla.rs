//! XLA HLO text emitter for nox formulas
//!
//! Compiles nox formulas to XLA HLO (High Level Operations) text format
//! for TPU and GPU execution via XLA runtime.
//!
//! XLA HLO is SSA — each instruction produces a named result. This maps
//! directly to nox's let-binding pattern. No special compose handling needed.
//!
//! All values are s64[] (64-bit signed integer scalars). Comparisons produce
//! pred[] intermediates, converted back to s64[] via select for nox semantics.
//!
//! Phase 1: atom-only formulas, no loops (HLO while → Phase 2).






use nox::{Reduction as Order, Order as NounId};
use super::{CompileError, formula_parts, body_pair, body_triple, atom_u64, axis_to_param};

/// Compile a nox formula to XLA HLO text format.
pub fn compile_to_xla<const N: usize>(
    order: &Order<N>,
    formula: NounId,
    num_params: u32,
) -> Result<String, CompileError> {
    let mut e = XlaEmitter::new(num_params);
    e.emit_formula(order, formula)?;
    let result = e.pop_reg();
    Ok(e.finish(&result, num_params))
}

struct XlaEmitter {
    body: String,
    next_var: u32,
    reg_stack: Vec<String>,
    subject: Vec<String>,
}

impl XlaEmitter {
    fn new(num_params: u32) -> Self {
        let subject: Vec<String> = (0..num_params).rev()
            .map(|i| format!("p{}", i))
            .collect();
        Self {
            body: String::with_capacity(2048),
            next_var: 0,
            reg_stack: Vec::new(),
            subject,
        }
    }

    fn alloc_var(&mut self) -> String {
        let v = format!("t{}", self.next_var);
        self.next_var += 1;
        v
    }

    fn push_reg(&mut self) -> String {
        let v = self.alloc_var();
        self.reg_stack.push(v.clone());
        v
    }

    fn pop_reg(&mut self) -> String {
        self.reg_stack.pop().unwrap_or_else(|| "t0".into())
    }

    fn emit(&mut self, line: &str) {
        self.body.push_str("  ");
        self.body.push_str(line);
        self.body.push('\n');
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
        // In HLO SSA, alias by emitting a no-op add with zero
        let dst = self.push_reg();
        let zero = self.alloc_var();
        self.emit(&format!("{} = s64[] constant(0)", zero));
        self.emit(&format!("{} = s64[] add({}, {})", dst, src, zero));
        Ok(())
    }

    fn emit_quote<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let val = atom_u64(order, body)?;
        let dst = self.push_reg();
        self.emit(&format!("{} = s64[] constant({})", dst, val));
        Ok(())
    }

    fn emit_compose<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        // Let-binding: [2 [[3 [value_formula [0 1]]] [1 body_formula]]]
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

        // Evaluate value, bind into subject, evaluate body
        self.emit_formula(order, value_formula)?;
        let val = self.pop_reg();
        self.subject.insert(0, val);
        let result = self.emit_formula(order, body_formula);
        self.subject.remove(0);
        result
    }

    fn emit_branch<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (test, yes, no) = body_triple(order, body)?;
        self.emit_formula(order, test)?;
        let test_r = self.pop_reg();
        self.emit_formula(order, yes)?;
        let yes_r = self.pop_reg();
        self.emit_formula(order, no)?;
        let no_r = self.pop_reg();

        // nox branch: 0=yes, nonzero=no
        let zero = self.alloc_var();
        let pred = self.alloc_var();
        let dst = self.push_reg();
        self.emit(&format!("{} = s64[] constant(0)", zero));
        self.emit(&format!("{} = pred[] compare({}, {}), direction=EQ", pred, test_r, zero));
        self.emit(&format!("{} = s64[] select({}, {}, {})", dst, pred, yes_r, no_r));
        Ok(())
    }

    fn emit_add<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("{} = s64[] add({}, {})", dst, ra, rb));
        Ok(())
    }

    fn emit_sub<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("{} = s64[] subtract({}, {})", dst, ra, rb));
        Ok(())
    }

    fn emit_mul<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("{} = s64[] multiply({}, {})", dst, ra, rb));
        Ok(())
    }

    fn emit_eq<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        // nox eq: 0 if equal, 1 if not
        let pred = self.alloc_var();
        let one = self.alloc_var();
        let zero = self.alloc_var();
        let dst = self.push_reg();
        self.emit(&format!("{} = pred[] compare({}, {}), direction=NE", pred, ra, rb));
        self.emit(&format!("{} = s64[] constant(1)", one));
        self.emit(&format!("{} = s64[] constant(0)", zero));
        self.emit(&format!("{} = s64[] select({}, {}, {})", dst, pred, one, zero));
        Ok(())
    }

    fn emit_lt<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        // nox lt: 0 if a<b, 1 if a>=b
        let pred = self.alloc_var();
        let one = self.alloc_var();
        let zero = self.alloc_var();
        let dst = self.push_reg();
        self.emit(&format!("{} = pred[] compare({}, {}), direction=GE", pred, ra, rb));
        self.emit(&format!("{} = s64[] constant(1)", one));
        self.emit(&format!("{} = s64[] constant(0)", zero));
        self.emit(&format!("{} = s64[] select({}, {}, {})", dst, pred, one, zero));
        Ok(())
    }

    fn emit_xor<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("{} = s64[] xor({}, {})", dst, ra, rb));
        Ok(())
    }

    fn emit_and<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("{} = s64[] and({}, {})", dst, ra, rb));
        Ok(())
    }

    fn emit_not<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        self.emit_formula(order, body)?;
        let ra = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("{} = s64[] not({})", dst, ra));
        // Mask to 32 bits (same as other emitters)
        let mask = self.alloc_var();
        let masked = self.alloc_var();
        self.emit(&format!("{} = s64[] constant(4294967295)", mask));
        // Replace dst on stack with masked result
        self.reg_stack.pop();
        self.reg_stack.push(masked.clone());
        self.emit(&format!("{} = s64[] and({}, {})", masked, dst, mask));
        Ok(())
    }

    fn emit_shl<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let shifted = self.alloc_var();
        let mask = self.alloc_var();
        let dst = self.push_reg();
        self.emit(&format!("{} = s64[] shift-left({}, {})", shifted, ra, rb));
        self.emit(&format!("{} = s64[] constant(4294967295)", mask));
        self.emit(&format!("{} = s64[] and({}, {})", dst, shifted, mask));
        Ok(())
    }

    fn finish(self, result: &str, num_params: u32) -> String {
        let mut hlo = String::with_capacity(4096);

        // Module header with layout
        hlo.push_str("HloModule nox_formula");
        if num_params > 0 {
            hlo.push_str(", entry_computation_layout={");
            hlo.push('(');
            for i in 0..num_params {
                if i > 0 { hlo.push(','); }
                hlo.push_str("s64[]");
            }
            hlo.push_str(")->s64[]");
            hlo.push('}');
        }
        hlo.push_str("\n\n");

        // Entry computation
        hlo.push_str("ENTRY main {\n");

        // Parameters
        for i in 0..num_params {
            hlo.push_str(&format!("  p{} = s64[] parameter({})\n", i, i));
        }

        // Body (already indented by emit())
        hlo.push_str(&self.body);

        // Mark the last instruction as ROOT by finding and replacing it
        // The result variable's definition line needs ROOT prefix
        let root_prefix = format!("  {} = ", result);
        if let Some(pos) = hlo.rfind(&root_prefix) {
            hlo.replace_range(pos..pos + root_prefix.len(),
                              &format!("  ROOT {} = ", result));
        }

        hlo.push_str("}\n");
        hlo
    }
}
