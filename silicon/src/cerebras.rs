//! Cerebras CSL text emitter for nox formulas
//!
//! Compiles nox formulas to Cerebras Software Language (CSL) source code
//! for execution on the Cerebras CS-2 wafer-scale engine (850K cores).
//!
//! CSL syntax is Zig-like. Each core runs one task. Parameters arrive via
//! fabric (inter-core communication), result sent back via fabric.
//! The host distributes work across cores.
//!
//! All values are u32. No Goldilocks reduction — CSL operates on raw
//! 32-bit integers. Phase 1: atom-only formulas, no loops.






use nox::{Reduction as Order, Order as NounId};
use super::{CompileError, formula_parts, body_pair, body_triple, atom_u64, axis_to_param};

/// Compile a nox formula to Cerebras CSL source text.
pub fn compile_to_csl<const N: usize>(
    order: &Order<N>,
    formula: NounId,
    num_params: u32,
) -> Result<String, CompileError> {
    let mut e = CslEmitter::new(num_params);
    e.emit_formula(order, formula)?;
    let result = e.pop_reg();
    Ok(e.finish(&result, num_params))
}

struct CslEmitter {
    body: String,
    next_var: u32,
    reg_stack: Vec<String>,
    subject: Vec<String>,
}

impl CslEmitter {
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
        self.body.push_str("    ");
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
        let dst = self.push_reg();
        self.emit(&format!("const {}: u32 = {};", dst, src));
        Ok(())
    }

    fn emit_quote<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let val = atom_u64(order, body)?;
        let dst = self.push_reg();
        self.emit(&format!("const {}: u32 = {};", dst, val as u32));
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
        let dst = self.push_reg();
        self.emit(&format!("const {}: u32 = if ({} == 0) {} else {};", dst, test_r, yes_r, no_r));
        Ok(())
    }

    fn emit_add<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("const {}: u32 = {} + {};", dst, ra, rb));
        Ok(())
    }

    fn emit_sub<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("const {}: u32 = {} - {};", dst, ra, rb));
        Ok(())
    }

    fn emit_mul<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("const {}: u32 = {} * {};", dst, ra, rb));
        Ok(())
    }

    fn emit_eq<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        // nox eq: 0 if equal, 1 if not
        let dst = self.push_reg();
        self.emit(&format!("const {}: u32 = if ({} != {}) 1 else 0;", dst, ra, rb));
        Ok(())
    }

    fn emit_lt<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        // nox lt: 0 if a<b, 1 if a>=b
        let dst = self.push_reg();
        self.emit(&format!("const {}: u32 = if ({} >= {}) 1 else 0;", dst, ra, rb));
        Ok(())
    }

    fn emit_xor<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("const {}: u32 = {} ^ {};", dst, ra, rb));
        Ok(())
    }

    fn emit_and<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("const {}: u32 = {} & {};", dst, ra, rb));
        Ok(())
    }

    fn emit_not<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        self.emit_formula(order, body)?;
        let ra = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("const {}: u32 = ~{};", dst, ra));
        Ok(())
    }

    fn emit_shl<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        // CSL uses @as for type-punning shift amount to u5
        self.emit(&format!("const {}: u32 = {} << @as(u5, {});", dst, ra, rb));
        Ok(())
    }

    /// Wrap body in a complete CSL task that receives params via fabric.
    fn finish(self, result: &str, num_params: u32) -> String {
        let mut csl = String::with_capacity(4096);

        // Module imports
        csl.push_str("const std = @import(\"std\");\n\n");

        // Task entry point
        csl.push_str("task main() void {\n");

        // Receive parameters from fabric
        for i in 0..num_params {
            csl.push_str(&format!("    const p{}: u32 = @as(u32, fabric_recv());\n", i));
        }
        if num_params > 0 {
            csl.push('\n');
        }

        // Formula body
        csl.push_str(&self.body);

        // Send result back via fabric
        csl.push_str(&format!("\n    fabric_send({});\n", result));
        csl.push_str("}\n");
        csl
    }
}
