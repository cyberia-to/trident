//! ONNX model emitter for nox formulas
//!
//! Generates ONNX protobuf binary by hand — no protobuf dependency.
//! All tensors: shape [1], data_type INT64. Maps nox operations to
//! ONNX operators (Add, Sub, Mul, Equal, Less, BitwiseXor, BitwiseAnd, Where).
//!
//! Wire format: Protocol Buffers encoding emitted directly into Vec<u8>.
//! ir_version = 9, opset_import version = 19.





use nox::{Reduction as Order, Order as NounId};
use super::{CompileError, formula_parts, body_pair, body_triple, atom_u64, axis_to_param,
            detect_loop_setup, detect_back_edge};

/// Compile nox formula to ONNX model bytes.
/// Params become graph inputs (INT64 scalar tensors), result is a single graph output.
pub fn compile_to_onnx<const N: usize>(
    order: &Order<N>,
    formula: NounId,
    num_params: u32,
) -> Result<Vec<u8>, CompileError> {
    let mut e = OnnxEmitter::new(num_params);
    e.emit_formula(order, formula)?;
    let result = e.pop_reg();
    Ok(e.finish(&result, num_params))
}

// ── Protobuf wire format helpers ────────────────────────────────

fn encode_varint(buf: &mut Vec<u8>, mut val: u64) {
    loop {
        let byte = (val & 0x7F) as u8;
        val >>= 7;
        if val == 0 {
            buf.push(byte);
            break;
        }
        buf.push(byte | 0x80);
    }
}

fn encode_field_varint(buf: &mut Vec<u8>, field: u32, val: u64) {
    encode_varint(buf, ((field as u64) << 3) | 0); // wire type 0
    encode_varint(buf, val);
}

fn encode_field_bytes(buf: &mut Vec<u8>, field: u32, data: &[u8]) {
    encode_varint(buf, ((field as u64) << 3) | 2); // wire type 2
    encode_varint(buf, data.len() as u64);
    buf.extend_from_slice(data);
}

fn encode_field_string(buf: &mut Vec<u8>, field: u32, s: &str) {
    encode_field_bytes(buf, field, s.as_bytes());
}

// ── ONNX protobuf message builders ─────────────────────────────

/// TensorProto.DataType for INT64
const DT_INT64: u64 = 7;

/// Encode TensorShapeProto.Dimension (field 1 = dim_value as varint, field tag 1)
fn encode_tensor_shape_dim(dim: i64) -> Vec<u8> {
    let mut buf = Vec::new();
    // dim_value is field 1, varint
    encode_field_varint(&mut buf, 1, dim as u64);
    buf
}

/// Encode TensorShapeProto (field 1 = dim[], repeated)
fn encode_tensor_shape(dims: &[i64]) -> Vec<u8> {
    let mut buf = Vec::new();
    for &d in dims {
        let dim_bytes = encode_tensor_shape_dim(d);
        encode_field_bytes(&mut buf, 1, &dim_bytes);
    }
    buf
}

/// Encode TypeProto.Tensor (field 1 = elem_type, field 2 = shape)
fn encode_type_tensor(elem_type: u64, dims: &[i64]) -> Vec<u8> {
    let mut buf = Vec::new();
    encode_field_varint(&mut buf, 1, elem_type);
    let shape = encode_tensor_shape(dims);
    encode_field_bytes(&mut buf, 2, &shape);
    buf
}

/// Encode TypeProto (field 1 = tensor_type as embedded message)
fn encode_type_proto(elem_type: u64, dims: &[i64]) -> Vec<u8> {
    let mut buf = Vec::new();
    let tensor = encode_type_tensor(elem_type, dims);
    encode_field_bytes(&mut buf, 1, &tensor);
    buf
}

/// Encode ValueInfoProto (field 1 = name, field 2 = type)
fn encode_value_info(name: &str, elem_type: u64, dims: &[i64]) -> Vec<u8> {
    let mut buf = Vec::new();
    encode_field_string(&mut buf, 1, name);
    let tp = encode_type_proto(elem_type, dims);
    encode_field_bytes(&mut buf, 2, &tp);
    buf
}

/// Encode TensorProto for an INT64 scalar constant (used in initializers).
/// Fields: 1=dims[], 2=data_type, 13=name, 5=int64_data[] (packed)
fn encode_tensor_int64(name: &str, value: i64) -> Vec<u8> {
    let mut buf = Vec::new();
    // dims: [1] — field 1, varint, repeated
    encode_field_varint(&mut buf, 1, 1);
    // data_type: INT64 = 7 — field 2
    encode_field_varint(&mut buf, 2, DT_INT64);
    // int64_data: packed repeated — field 5, wire type 2
    let mut packed = Vec::new();
    encode_varint(&mut packed, value as u64);
    encode_field_bytes(&mut buf, 5, &packed);
    // name: field 13 (0x0D in decimal, not hex)
    encode_field_string(&mut buf, 13, name);
    buf
}

/// Encode a NodeProto.
/// Fields: 1=input[] (repeated string), 2=output[] (repeated string),
///         4=op_type (string), 7=domain (string)
fn encode_node(op_type: &str, inputs: &[&str], outputs: &[&str], attrs: &[(&str, i64)]) -> Vec<u8> {
    let mut buf = Vec::new();
    for inp in inputs {
        encode_field_string(&mut buf, 1, inp);
    }
    for out in outputs {
        encode_field_string(&mut buf, 2, out);
    }
    encode_field_string(&mut buf, 4, op_type);
    // Attributes (field 5, AttributeProto messages)
    for &(attr_name, attr_val) in attrs {
        let attr = encode_attribute_int(attr_name, attr_val);
        encode_field_bytes(&mut buf, 5, &attr);
    }
    buf
}

/// Encode an AttributeProto with int value.
/// Fields: 1=name (string), 2=ref_attr_name, 3=i (int), 4=doc_string,
///         20=type (enum: 2=INT)
fn encode_attribute_int(name: &str, value: i64) -> Vec<u8> {
    let mut buf = Vec::new();
    encode_field_string(&mut buf, 1, name);
    encode_field_varint(&mut buf, 3, value as u64);
    encode_field_varint(&mut buf, 20, 2); // AttributeType INT = 2
    buf
}

/// Encode OpsetImportProto. Fields: 1=domain (string), 2=version (int64)
fn encode_opset_import(domain: &str, version: u64) -> Vec<u8> {
    let mut buf = Vec::new();
    if !domain.is_empty() {
        encode_field_string(&mut buf, 1, domain);
    }
    encode_field_varint(&mut buf, 2, version);
    buf
}

/// Encode GraphProto.
/// Fields: 1=node[] (repeated), 11=name, 12=input[], 13=initializer[], 14=output[]
fn encode_graph(
    name: &str,
    nodes: &[Vec<u8>],
    inputs: &[Vec<u8>],
    outputs: &[Vec<u8>],
    initializers: &[Vec<u8>],
) -> Vec<u8> {
    let mut buf = Vec::new();
    for n in nodes {
        encode_field_bytes(&mut buf, 1, n);
    }
    encode_field_string(&mut buf, 11, name);
    for inp in inputs {
        encode_field_bytes(&mut buf, 12, inp);
    }
    for init in initializers {
        encode_field_bytes(&mut buf, 13, init);
    }
    for out in outputs {
        encode_field_bytes(&mut buf, 14, out);
    }
    buf
}

/// Encode ModelProto.
/// Fields: 1=ir_version (int64), 7=graph (GraphProto), 8=opset_import[] (repeated)
fn encode_model(ir_version: u64, graph: &[u8], opset_imports: &[Vec<u8>]) -> Vec<u8> {
    let mut buf = Vec::new();
    encode_field_varint(&mut buf, 1, ir_version);
    for osi in opset_imports {
        encode_field_bytes(&mut buf, 8, osi);
    }
    encode_field_bytes(&mut buf, 7, graph);
    buf
}

// ── Emitter ─────────────────────────────────────────────────────

struct OnnxEmitter {
    nodes: Vec<Vec<u8>>,
    initializers: Vec<Vec<u8>>,
    init_value_infos: Vec<Vec<u8>>,
    next_var: u32,
    reg_stack: Vec<String>,
    subject: Vec<String>,
}

impl OnnxEmitter {
    fn new(num_params: u32) -> Self {
        let subject: Vec<String> = (0..num_params).rev()
            .map(|i| format!("p{}", i)).collect();
        Self {
            nodes: Vec::new(),
            initializers: Vec::new(),
            init_value_infos: Vec::new(),
            next_var: 0,
            reg_stack: Vec::new(),
            subject,
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

    /// Add a constant initializer (INT64 scalar).
    fn add_const(&mut self, name: &str, value: i64) {
        self.initializers.push(encode_tensor_int64(name, value));
        self.init_value_infos.push(encode_value_info(name, DT_INT64, &[1]));
    }

    /// Add a computation node.
    fn add_node(&mut self, op_type: &str, inputs: &[&str], outputs: &[&str]) {
        self.nodes.push(encode_node(op_type, inputs, outputs, &[]));
    }

    /// Add a computation node with attributes.
    fn add_node_with_attrs(&mut self, op_type: &str, inputs: &[&str], outputs: &[&str], attrs: &[(&str, i64)]) {
        self.nodes.push(encode_node(op_type, inputs, outputs, attrs));
    }

    fn emit_formula<const N: usize>(&mut self, order: &Order<N>, formula: NounId) -> Result<(), CompileError> {
        let (tag, body) = formula_parts(order, formula)?;
        match tag {
            0 => self.emit_axis(order, body),
            1 => self.emit_quote(order, body),
            2 => self.emit_compose(order, body),
            4 => self.emit_branch(order, body),
            5 => self.emit_binop(order, body, "Add"),
            6 => self.emit_binop(order, body, "Sub"),
            7 => self.emit_binop(order, body, "Mul"),
            9 => self.emit_eq(order, body),
            10 => self.emit_lt(order, body),
            11 => self.emit_binop(order, body, "BitwiseXor"),
            12 => self.emit_binop(order, body, "BitwiseAnd"),
            13 => self.emit_not(order, body),
            14 => self.emit_shift(order, body),
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
        let name = self.alloc_var("c");
        self.add_const(&name, val as i64);
        self.reg_stack.push(name);
        Ok(())
    }

    fn emit_compose<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        if let Some((_loop_body, _inits)) = detect_loop_setup(order, body) {
            return Err(CompileError::UnsupportedPattern(2));
        }
        if let Some((_new_subj, _)) = detect_back_edge(order, body) {
            return Err(CompileError::UnsupportedPattern(2));
        }
        // Let-binding: [3 [[1 value] [0 1]]] composed with [1 body]
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
        self.add_node(op, &[&ra, &rb], &[&dst]);
        Ok(())
    }

    fn emit_eq<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        // nox eq: 0 if equal, 1 if not
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        // Equal → bool, then negate (Not), then Cast to INT64
        let eq_bool = self.alloc_var("eq");
        self.add_node("Equal", &[&ra, &rb], &[&eq_bool]);
        let neq_bool = self.alloc_var("neq");
        self.add_node("Not", &[&eq_bool], &[&neq_bool]);
        let dst = self.push_reg();
        // Cast bool → INT64 (to=7)
        self.add_node_with_attrs("Cast", &[&neq_bool], &[&dst], &[("to", DT_INT64 as i64)]);
        Ok(())
    }

    fn emit_lt<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        // nox lt: 0 if a<b, 1 if a>=b
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        // Less → bool (true if a<b), then Not (nox: 0=true), then Cast
        let lt_bool = self.alloc_var("lt");
        self.add_node("Less", &[&ra, &rb], &[&lt_bool]);
        let nlt_bool = self.alloc_var("nlt");
        self.add_node("Not", &[&lt_bool], &[&nlt_bool]);
        let dst = self.push_reg();
        self.add_node_with_attrs("Cast", &[&nlt_bool], &[&dst], &[("to", DT_INT64 as i64)]);
        Ok(())
    }

    fn emit_not<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        self.emit_formula(order, body)?;
        let ra = self.pop_reg();
        // Bitwise NOT via XOR with 0xFFFFFFFF
        let mask_name = self.alloc_var("mask");
        self.add_const(&mask_name, 0xFFFF_FFFF_i64);
        let xor_out = self.alloc_var("xnot");
        self.add_node("BitwiseXor", &[&ra, &mask_name], &[&xor_out]);
        // AND with mask to keep 32-bit
        let dst = self.push_reg();
        self.add_node("BitwiseAnd", &[&xor_out, &mask_name], &[&dst]);
        Ok(())
    }

    fn emit_shift<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        // nox shift left (tag 14)
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        // ONNX BitShift op: direction attribute "LEFT"
        // BitShift uses uint types; for INT64 this works on the bit pattern
        let dst = self.push_reg();
        // BitShift not available for signed int64 in all runtimes.
        // Use Mul(a, Pow(2, b)) as portable fallback.
        let two = self.alloc_var("two");
        self.add_const(&two, 2);
        let pow_out = self.alloc_var("pow");
        self.add_node("Pow", &[&two, &rb], &[&pow_out]);
        self.add_node("Mul", &[&ra, &pow_out], &[&dst]);
        Ok(())
    }

    fn emit_branch<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        // nox branch: 0=yes, nonzero=no → Where(condition, yes, no)
        let (test, yes, no) = body_triple(order, body)?;
        self.emit_formula(order, test)?;
        let test_r = self.pop_reg();
        self.emit_formula(order, yes)?;
        let yr = self.pop_reg();
        self.emit_formula(order, no)?;
        let nr = self.pop_reg();

        // test == 0 → choose yes, else no
        let zero = self.alloc_var("zero");
        self.add_const(&zero, 0);
        let cond = self.alloc_var("cond");
        self.add_node("Equal", &[&test_r, &zero], &[&cond]);
        // Where(cond, yes, no): picks yes where cond=true
        let dst = self.push_reg();
        self.add_node("Where", &[&cond, &yr, &nr], &[&dst]);
        Ok(())
    }

    // ── ONNX model assembly ─────────────────────────────────────

    fn finish(self, result: &str, num_params: u32) -> Vec<u8> {
        // Build graph inputs (ValueInfoProto for each parameter)
        let mut inputs: Vec<Vec<u8>> = Vec::new();
        for i in 0..num_params {
            let name = format!("p{}", i);
            inputs.push(encode_value_info(&name, DT_INT64, &[1]));
        }
        // Also add initializer ValueInfoProtos as inputs (ONNX requires this)
        for vi in &self.init_value_infos {
            inputs.push(vi.clone());
        }

        // Graph output
        let outputs = vec![encode_value_info(result, DT_INT64, &[1])];

        // Encode graph
        let graph = encode_graph(
            "nox_graph",
            &self.nodes,
            &inputs,
            &outputs,
            &self.initializers,
        );

        // Opset import: default domain, version 19
        let opset = encode_opset_import("", 19);

        // Encode model
        encode_model(9, &graph, &[opset])
    }
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
    fn onnx_quote_compiles() {
        let mut order = Order::<1024>::new();
        let body = make_atom(&mut order, 42);
        let formula = make_formula(&mut order, 1, body);
        let onnx = compile_to_onnx(&order, formula, 0).unwrap();
        // Should produce non-empty protobuf bytes
        assert!(onnx.len() > 10);
        // First byte should be a protobuf field tag (field 1 varint = 0x08)
        assert_eq!(onnx[0], 0x08, "should start with ir_version field");
    }

    #[test]
    fn onnx_add_compiles() {
        let mut o = Order::<1024>::new();
        let a3 = make_atom(&mut o, 3);
        let q3 = make_formula(&mut o, 1, a3);
        let a5 = make_atom(&mut o, 5);
        let q5 = make_formula(&mut o, 1, a5);
        let formula = make_binary(&mut o, 5, q3, q5);
        let onnx = compile_to_onnx(&o, formula, 0).unwrap();
        assert!(onnx.len() > 10);
    }

    #[test]
    fn onnx_axis_param_compiles() {
        let mut o = Order::<1024>::new();
        let ax2 = make_atom(&mut o, 2);
        let a0 = make_formula(&mut o, 0, ax2);
        let ax6 = make_atom(&mut o, 6);
        let a1 = make_formula(&mut o, 0, ax6);
        let formula = make_binary(&mut o, 5, a0, a1);
        let onnx = compile_to_onnx(&o, formula, 2).unwrap();
        assert!(onnx.len() > 10);
    }

    #[test]
    fn onnx_branch_compiles() {
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
        let onnx = compile_to_onnx(&o, formula, 0).unwrap();
        assert!(onnx.len() > 10);
    }

    #[test]
    fn onnx_eq_compiles() {
        let mut o = Order::<1024>::new();
        let a3 = make_atom(&mut o, 3);
        let q3 = make_formula(&mut o, 1, a3);
        let a5 = make_atom(&mut o, 5);
        let q5 = make_formula(&mut o, 1, a5);
        let formula = make_binary(&mut o, 9, q3, q5);
        let onnx = compile_to_onnx(&o, formula, 0).unwrap();
        assert!(onnx.len() > 10);
    }

    #[test]
    fn onnx_mul_compiles() {
        let mut o = Order::<1024>::new();
        let ax2 = make_atom(&mut o, 2);
        let a0 = make_formula(&mut o, 0, ax2);
        let ax6 = make_atom(&mut o, 6);
        let a1 = make_formula(&mut o, 0, ax6);
        let formula = make_binary(&mut o, 7, a0, a1);
        let onnx = compile_to_onnx(&o, formula, 2).unwrap();
        assert!(onnx.len() > 10);
    }

    #[test]
    fn onnx_unsupported_pattern() {
        let mut o = Order::<1024>::new();
        let a1 = make_atom(&mut o, 1);
        let body = make_formula(&mut o, 0, a1);
        let formula = make_formula(&mut o, 15, body);
        assert!(matches!(
            compile_to_onnx(&o, formula, 1),
            Err(CompileError::UnsupportedPattern(15))
        ));
    }

    #[test]
    fn varint_encoding() {
        // Single byte
        let mut buf = Vec::new();
        encode_varint(&mut buf, 0);
        assert_eq!(buf, [0]);

        buf.clear();
        encode_varint(&mut buf, 127);
        assert_eq!(buf, [127]);

        // Two bytes: 128 = 0x80 → 0x00 0x01 in varint
        buf.clear();
        encode_varint(&mut buf, 128);
        assert_eq!(buf, [0x80, 0x01]);

        // 300 = 0x12C → varint bytes
        buf.clear();
        encode_varint(&mut buf, 300);
        assert_eq!(buf, [0xAC, 0x02]);
    }
}
