//! Apple Neural Engine (ANE) MIL emitter for nox formulas
//!
//! Generates MIL text for Apple's Neural Engine via rane.
//! ANE operates in fp16 — integer field arithmetic stays on CPU/GPU.
//! Neural operations (matmul, add, mul) map to ANE hardware.
//!
//! Two modes:
//!   - Element-wise: each nox op → one MIL operation on tensors
//!   - Batch: formula evaluated across spatial dimension (one ANE dispatch)
//!
//! Limitations:
//!   - fp16 only (no Goldilocks u64 — use for neural inference, not field ops)
//!   - ANE rejects: reduce_mean, rsqrt, reduce_sum, pow
//!   - All tensors 4D: [1, C, 1, S]






use nox::noun::{Order, NounId};
use super::{CompileError, formula_parts, body_pair, body_triple, atom_u64, axis_to_param,
            detect_loop_setup, detect_back_edge};

/// Compile nox formula to MIL text for ANE.
/// Params become channels in a single input tensor [1, num_params, 1, 1].
/// Result is output tensor [1, 1, 1, 1].
pub fn compile_to_mil<const N: usize>(
    order: &Order<N>,
    formula: NounId,
    num_params: u32,
) -> Result<String, CompileError> {
    let mut e = MilEmitter::new(num_params);
    e.emit_formula(order, formula)?;
    let result = e.pop_reg();
    Ok(e.finish(&result, num_params))
}

/// Compile for batch processing: formula applied across spatial dimension.
/// Input: [1, num_params, 1, batch_size], Output: [1, 1, 1, batch_size].
/// Each spatial position = one independent evaluation.
pub fn compile_to_mil_batch<const N: usize>(
    order: &Order<N>,
    formula: NounId,
    num_params: u32,
    batch_size: u32,
) -> Result<String, CompileError> {
    let mut e = MilEmitter::new(num_params);
    e.emit_formula(order, formula)?;
    let result = e.pop_reg();
    Ok(e.finish_batch(&result, num_params, batch_size))
}

struct MilEmitter {
    body: String,
    num_params: u32,
    next_var: u32,
    reg_stack: Vec<String>,
    subject: Vec<String>,
    loop_state: Option<MilLoopState>,
}

#[derive(Clone)]
struct MilLoopState {
    carried: Vec<String>,
}

impl MilEmitter {
    fn new(num_params: u32) -> Self {
        let subject: Vec<String> = (0..num_params).rev()
            .map(|i| format!("p{}", i)).collect();
        Self {
            body: String::with_capacity(4096),
            num_params, next_var: 0,
            reg_stack: Vec::new(), subject,
            loop_state: None,
        }
    }

    fn alloc_var(&mut self, prefix: &str) -> String {
        let v = format!("{}_{}", prefix, self.next_var);
        self.next_var += 1;
        v
    }

    fn push_reg(&mut self) -> String {
        let v = self.alloc_var("t");
        self.reg_stack.push(v.clone());
        v
    }

    fn pop_reg(&mut self) -> String {
        self.reg_stack.pop().unwrap_or_else(|| "t_0".into())
    }

    fn emit(&mut self, line: &str) {
        self.body.push_str("        ");
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
            5 => self.emit_binop(order, body, "add"),
            6 => self.emit_binop(order, body, "sub"),
            7 => self.emit_binop(order, body, "mul"),
            9 => self.emit_eq(order, body),
            10 => self.emit_lt(order, body),
            _ => Err(CompileError::UnsupportedPattern(tag)),
        }
    }

    fn emit_axis<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let addr = atom_u64(order, body)?;
        let depth = axis_to_param(addr)?;
        if (depth as usize) >= self.subject.len() { return Err(CompileError::NoParams); }
        let src = self.subject[depth as usize].clone();
        self.reg_stack.push(src);
        Ok(())
    }

    fn emit_quote<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let val = atom_u64(order, body)?;
        let dst = self.push_reg();
        let fp = val as f32; // convert to fp16-compatible float
        self.emit(&format!(
            "tensor<fp16, [1,1,1,1]> {} = const()[name=string(\"{}\"), val=tensor<fp16, [1,1,1,1]>([{}])];",
            dst, dst, fp
        ));
        Ok(())
    }

    fn emit_compose<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        if let Some((_loop_body, _inits)) = detect_loop_setup(order, body) {
            return Err(CompileError::UnsupportedPattern(2)); // ANE: no loops in MIL
        }
        if let Some((_new_subj, _)) = detect_back_edge(order, body) {
            return Err(CompileError::UnsupportedPattern(2));
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

    fn emit_binop<const N: usize>(&mut self, order: &Order<N>, body: NounId, op: &str) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!(
            "tensor<fp16, [1,1,1,1]> {} = {}(x={}, y={})[name=string(\"{}\")];",
            dst, op, ra, rb, dst
        ));
        Ok(())
    }

    fn emit_eq<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        // ANE doesn't have eq directly. Use: sub, then check if zero.
        // Approximate: abs(a-b) < epsilon → 0, else 1
        // For fp16 neural use, exact equality is rare. Emit sub for now.
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let diff = self.alloc_var("eq");
        let dst = self.push_reg();
        self.emit(&format!(
            "tensor<fp16, [1,1,1,1]> {} = sub(x={}, y={})[name=string(\"{}\")];",
            diff, ra, rb, diff
        ));
        // Approximate: sigmoid(large * diff) pushes toward 0 or 1
        // For exact: not possible in fp16. Return diff as indicator.
        self.emit(&format!(
            "tensor<fp16, [1,1,1,1]> {} = mul(x={}, y={})[name=string(\"{}\")];",
            dst, diff, diff, dst  // squared diff: 0 if equal
        ));
        Ok(())
    }

    fn emit_lt<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        // ANE: no comparison ops. Approximate via sub + sigmoid.
        // sigmoid(100 * (b - a)) ≈ 1 if a < b, 0 if a >= b
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let diff = self.alloc_var("lt");
        let scaled = self.alloc_var("lt");
        let scale_c = self.alloc_var("lt");
        let dst = self.push_reg();
        self.emit(&format!(
            "tensor<fp16, [1,1,1,1]> {} = sub(x={}, y={})[name=string(\"{}\")];",
            diff, rb, ra, diff  // b - a: positive if a < b
        ));
        self.emit(&format!(
            "tensor<fp16, [1,1,1,1]> {} = const()[name=string(\"{}\"), val=tensor<fp16, [1,1,1,1]>([100.0])];",
            scale_c, scale_c
        ));
        self.emit(&format!(
            "tensor<fp16, [1,1,1,1]> {} = mul(x={}, y={})[name=string(\"{}\")];",
            scaled, diff, scale_c, scaled
        ));
        self.emit(&format!(
            "tensor<fp16, [1,1,1,1]> {} = sigmoid(x={})[name=string(\"{}\")];",
            dst, scaled, dst  // ≈1 if a<b, ≈0 if a>=b
        ));
        Ok(())
    }

    fn emit_branch<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        // ANE: no control flow. Approximate via: result = test * no + (1 - test) * yes
        // nox: 0=yes, nonzero=no
        let (test, yes, no) = body_triple(order, body)?;
        self.emit_formula(order, test)?;
        let test_r = self.pop_reg();
        self.emit_formula(order, yes)?;
        let yr = self.pop_reg();
        self.emit_formula(order, no)?;
        let nr = self.pop_reg();

        let one = self.alloc_var("br");
        let inv = self.alloc_var("br");
        let yes_part = self.alloc_var("br");
        let no_part = self.alloc_var("br");
        let dst = self.push_reg();

        self.emit(&format!(
            "tensor<fp16, [1,1,1,1]> {} = const()[name=string(\"{}\"), val=tensor<fp16, [1,1,1,1]>([1.0])];",
            one, one
        ));
        // Clamp test to [0,1]: sigmoid(large * test)
        let clamped = self.alloc_var("br");
        let scale = self.alloc_var("br");
        self.emit(&format!(
            "tensor<fp16, [1,1,1,1]> {} = const()[name=string(\"{}\"), val=tensor<fp16, [1,1,1,1]>([100.0])];",
            scale, scale
        ));
        self.emit(&format!(
            "tensor<fp16, [1,1,1,1]> {} = mul(x={}, y={})[name=string(\"{}\")];",
            clamped, test_r, scale, clamped
        ));
        let sig = self.alloc_var("br");
        self.emit(&format!(
            "tensor<fp16, [1,1,1,1]> {} = sigmoid(x={})[name=string(\"{}\")];",
            sig, clamped, sig
        ));
        // inv = 1 - sig
        self.emit(&format!(
            "tensor<fp16, [1,1,1,1]> {} = sub(x={}, y={})[name=string(\"{}\")];",
            inv, one, sig, inv
        ));
        // result = inv * yes + sig * no
        self.emit(&format!(
            "tensor<fp16, [1,1,1,1]> {} = mul(x={}, y={})[name=string(\"{}\")];",
            yes_part, inv, yr, yes_part
        ));
        self.emit(&format!(
            "tensor<fp16, [1,1,1,1]> {} = mul(x={}, y={})[name=string(\"{}\")];",
            no_part, sig, nr, no_part
        ));
        self.emit(&format!(
            "tensor<fp16, [1,1,1,1]> {} = add(x={}, y={})[name=string(\"{}\")];",
            dst, yes_part, no_part, dst
        ));
        Ok(())
    }

    // ── MIL assembly ─────────────────────────────────────────────

    fn finish(self, result: &str, num_params: u32) -> String {
        let mut mil = String::with_capacity(8192);
        mil.push_str("program(1.3)\n");
        mil.push_str("[buildInfo = dict<string, string>({\"coremlc-component-MIL\", \"3510.2.1\"}, {\"coremlc-version\", \"3505.4.1\"}, {\"coremltools-component-milinternal\", \"\"}, {\"coremltools-version\", \"9.0\"})]\n");
        mil.push_str("{\n");
        mil.push_str(&format!(
            "    func main<ios18>(tensor<fp16, [1, {}, 1, 1]> x) {{\n",
            num_params
        ));

        // Slice each param from input channels
        for i in 0..num_params {
            mil.push_str(&format!(
                "        tensor<int32, [4]> p{i}_b = const()[name=string(\"p{i}_b\"), val=tensor<int32, [4]>([0,{i},0,0])];\n",
                i = i
            ));
            mil.push_str(&format!(
                "        tensor<int32, [4]> p{i}_s = const()[name=string(\"p{i}_s\"), val=tensor<int32, [4]>([1,1,1,1])];\n",
                i = i
            ));
            mil.push_str(&format!(
                "        tensor<fp16, [1,1,1,1]> p{i} = slice_by_size(x=x, begin=p{i}_b, size=p{i}_s)[name=string(\"p{i}\")];\n",
                i = i
            ));
        }

        mil.push_str(&self.body);
        mil.push_str(&format!("    }} -> ({});\n", result));
        mil.push_str("}\n");
        mil
    }

    fn finish_batch(self, result: &str, num_params: u32, batch: u32) -> String {
        let mut mil = String::with_capacity(8192);
        mil.push_str("program(1.3)\n");
        mil.push_str("[buildInfo = dict<string, string>({\"coremlc-component-MIL\", \"3510.2.1\"}, {\"coremlc-version\", \"3505.4.1\"}, {\"coremltools-component-milinternal\", \"\"}, {\"coremltools-version\", \"9.0\"})]\n");
        mil.push_str("{\n");
        mil.push_str(&format!(
            "    func main<ios18>(tensor<fp16, [1, {}, 1, {}]> x) {{\n",
            num_params, batch
        ));

        // Slice each param channel (full spatial/batch dimension)
        for i in 0..num_params {
            mil.push_str(&format!(
                "        tensor<int32, [4]> p{i}_b = const()[name=string(\"p{i}_b\"), val=tensor<int32, [4]>([0,{i},0,0])];\n", i = i
            ));
            mil.push_str(&format!(
                "        tensor<int32, [4]> p{i}_s = const()[name=string(\"p{i}_s\"), val=tensor<int32, [4]>([1,1,1,{b}])];\n", i = i, b = batch
            ));
            mil.push_str(&format!(
                "        tensor<fp16, [1,1,1,{b}]> p{i} = slice_by_size(x=x, begin=p{i}_b, size=p{i}_s)[name=string(\"p{i}\")];\n", i = i, b = batch
            ));
        }

        mil.push_str(&self.body);
        mil.push_str(&format!("    }} -> ({});\n", result));
        mil.push_str("}\n");
        mil
    }
}
