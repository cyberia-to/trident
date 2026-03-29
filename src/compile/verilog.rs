//! Verilog HDL emitter for nox formulas
//!
//! Compiles nox formulas into synthesizable Verilog for FPGA deployment.
//! Phase 1: pure combinational logic (no clock). Each nox operation becomes
//! a wire assignment. Loops are not supported (require sequential logic).
//!
//! All values are 64-bit unsigned in Goldilocks field (p = 2^64 - 2^32 + 1).
//! Arithmetic uses inline Goldilocks reduction.






use nox::noun::{Order, NounId};
use super::{CompileError, formula_parts, body_pair, body_triple, atom_u64, axis_to_param,
            detect_loop_setup, detect_back_edge};

const P: u64 = 0xFFFF_FFFF_0000_0001;

/// Compile a nox formula to synthesizable Verilog (combinational).
pub fn compile_to_verilog<const N: usize>(
    order: &Order<N>,
    formula: NounId,
    num_params: u32,
) -> Result<String, CompileError> {
    let mut e = VerilogEmitter::new(num_params);
    e.emit_formula(order, formula)?;
    let result = e.pop_reg();
    Ok(e.finish(&result, num_params))
}

struct VerilogEmitter {
    body: String,
    next_wire: u32,
    reg_stack: Vec<String>,
    subject: Vec<String>,
}

impl VerilogEmitter {
    fn new(num_params: u32) -> Self {
        let subject: Vec<String> = (0..num_params).rev()
            .map(|i| format!("p{}", i))
            .collect();
        Self {
            body: String::with_capacity(4096),
            next_wire: 0,
            reg_stack: Vec::new(),
            subject,
        }
    }

    fn alloc_wire(&mut self) -> String {
        let w = format!("t{}", self.next_wire);
        self.next_wire += 1;
        w
    }

    fn push_reg(&mut self) -> String {
        let w = self.alloc_wire();
        self.reg_stack.push(w.clone());
        w
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
        self.emit(&format!("wire [63:0] {} = {};", dst, src));
        Ok(())
    }

    fn emit_quote<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let val = atom_u64(order, body)?;
        let dst = self.push_reg();
        self.emit(&format!("wire [63:0] {} = 64'd{};", dst, val));
        Ok(())
    }

    fn emit_compose<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        // Loops require sequential logic — not supported in Phase 1
        if detect_loop_setup(order, body).is_some() {
            return Err(CompileError::UnsupportedPattern(2));
        }
        if detect_back_edge(order, body).is_some() {
            return Err(CompileError::UnsupportedPattern(2));
        }
        // Let-binding: [cons(value, identity) quote(body)]
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
        let test_w = self.pop_reg();
        self.emit_formula(order, yes)?;
        let yes_w = self.pop_reg();
        self.emit_formula(order, no)?;
        let no_w = self.pop_reg();
        let dst = self.push_reg();
        // nox: 0 = yes (true), nonzero = no (false)
        self.emit(&format!("wire [63:0] {} = ({} == 64'd0) ? {} : {};",
                           dst, test_w, yes_w, no_w));
        Ok(())
    }

    // ── Goldilocks arithmetic ──────────────────────────────────────

    fn emit_add<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();

        // 65-bit extended sum for overflow detection
        let sum_ext = self.alloc_wire();
        let overflow = self.alloc_wire();
        let sum_raw = self.alloc_wire();
        let sum_adj = self.alloc_wire();

        self.emit(&format!("wire [64:0] {} = {{1'b0, {}}} + {{1'b0, {}}};",
                           sum_ext, ra, rb));
        self.emit(&format!("wire {} = {}[64];", overflow, sum_ext));
        self.emit(&format!("wire [63:0] {} = {}[63:0];", sum_raw, sum_ext));
        // On overflow: add 2^32-1 (since P = 2^64 - 2^32 + 1, overflow means we subtracted 2^64 and must add back P,
        // but P = 2^64 - (2^32 - 1), so adding back (2^32-1) compensates for the lost 2^64 - P = 2^32-1)
        self.emit(&format!("wire [63:0] {} = {} ? ({} + 64'hFFFFFFFF) : {};",
                           sum_adj, overflow, sum_raw, sum_raw));
        // Final reduction: if sum_adj >= P, subtract P
        self.emit(&format!("wire [63:0] {} = ({} >= 64'h{:016X}) ? ({} - 64'h{:016X}) : {};",
                           dst, sum_adj, P, sum_adj, P, sum_adj));
        Ok(())
    }

    fn emit_sub<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();

        // If a >= b: result = a - b
        // If a < b:  result = P - b + a  (wraps around in field)
        let no_borrow = self.alloc_wire();
        let diff = self.alloc_wire();
        let wrap = self.alloc_wire();

        self.emit(&format!("wire {} = ({} >= {});", no_borrow, ra, rb));
        self.emit(&format!("wire [63:0] {} = {} - {};", diff, ra, rb));
        self.emit(&format!("wire [63:0] {} = 64'h{:016X} - {} + {};", wrap, P, rb, ra));
        self.emit(&format!("wire [63:0] {} = {} ? {} : {};",
                           dst, no_borrow, diff, wrap));
        Ok(())
    }

    fn emit_mul<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();

        // 128-bit product
        let prod = self.alloc_wire();
        let lo = self.alloc_wire();
        let hi = self.alloc_wire();

        self.emit(&format!("wire [127:0] {} = {} * {};", prod, ra, rb));
        self.emit(&format!("wire [63:0] {} = {}[63:0];", lo, prod));
        self.emit(&format!("wire [63:0] {} = {}[127:64];", hi, prod));

        // Goldilocks reduction: result = lo + hi * (2^32 - 1) mod P
        // hi*(2^32-1) = (hi << 32) - hi
        let hi_shift = self.alloc_wire();
        let hi_term = self.alloc_wire();
        let red_ext = self.alloc_wire();
        let red_overflow = self.alloc_wire();
        let red_raw = self.alloc_wire();
        let red_adj = self.alloc_wire();

        self.emit(&format!("wire [95:0] {} = {{{}[31:0], 32'd0}};", hi_shift, hi));
        self.emit(&format!("wire [95:0] {} = {} - {{32'd0, {}}};", hi_term, hi_shift, hi));

        // Add lo + hi_term (result fits in ~97 bits, use 96+1)
        self.emit(&format!("wire [96:0] {} = {{1'b0, {}}} + {{33'd0, {}}};",
                           red_ext, hi_term, lo));
        // Iterative reduction: while >= P, subtract P
        // For synthesizable Verilog, two conditional subtracts suffice
        self.emit(&format!("wire {} = ({}[96:64] != 33'd0) || ({}[63:0] >= 64'h{:016X});",
                           red_overflow, red_ext, red_ext, P));
        self.emit(&format!("wire [96:0] {} = {} ? ({} - {{33'd0, 64'h{:016X}}}) : {};",
                           red_raw, red_overflow, red_ext, P, red_ext));
        // Second reduction pass
        self.emit(&format!("wire [63:0] {} = ({}[63:0] >= 64'h{:016X}) ? ({}[63:0] - 64'h{:016X}) : {}[63:0];",
                           red_adj, red_raw, P, red_raw, P, red_raw));
        self.emit(&format!("wire [63:0] {} = {};", dst, red_adj));
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
        self.emit(&format!("wire [63:0] {} = ({} == {}) ? 64'd0 : 64'd1;",
                           dst, ra, rb));
        Ok(())
    }

    fn emit_lt<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        // nox lt: 0 if a < b, 1 if a >= b
        self.emit(&format!("wire [63:0] {} = ({} < {}) ? 64'd0 : 64'd1;",
                           dst, ra, rb));
        Ok(())
    }

    fn emit_xor<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("wire [63:0] {} = {} ^ {};", dst, ra, rb));
        Ok(())
    }

    fn emit_and<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("wire [63:0] {} = {} & {};", dst, ra, rb));
        Ok(())
    }

    fn emit_not<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        self.emit_formula(order, body)?;
        let ra = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("wire [63:0] {} = ~{};", dst, ra));
        Ok(())
    }

    fn emit_shl<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        // Use lower 6 bits of shift amount (max shift 63)
        self.emit(&format!("wire [63:0] {} = {} << {}[5:0];", dst, ra, rb));
        Ok(())
    }

    /// Assemble the complete Verilog module.
    fn finish(self, result: &str, num_params: u32) -> String {
        let mut v = String::with_capacity(8192);
        v.push_str("// nox formula → synthesizable Verilog (combinational)\n");
        v.push_str("// Goldilocks field: p = 2^64 - 2^32 + 1\n");
        v.push_str(&format!("// {} parameter(s), {} wire(s)\n\n", num_params, self.next_wire));

        v.push_str("module nox_formula (\n");
        for i in 0..num_params {
            v.push_str(&format!("    input  wire [63:0] p{},\n", i));
        }
        v.push_str("    output wire [63:0] result\n");
        v.push_str(");\n\n");

        // Wire declarations and assignments (the body)
        v.push_str(&self.body);

        // Final assignment
        v.push_str(&format!("\n    assign result = {};\n", result));
        v.push_str("\nendmodule\n");
        v
    }
}
