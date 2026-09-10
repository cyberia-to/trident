//! VHDL emitter for nox formulas
//!
//! Compiles nox formulas into synthesizable VHDL targeting aerospace and
//! mil-spec FPGA platforms (DO-254 / MIL-STD-882). Phase 1: pure
//! combinational logic (no clock). Each nox operation becomes a signal
//! assignment. Loops are not supported (require sequential logic).
//!
//! All values are 64-bit unsigned in Goldilocks field (p = 2^64 - 2^32 + 1).
//! Arithmetic uses inline Goldilocks reduction.

use nox::{Reduction as Order, Order as NounId};
use super::{CompileError, formula_parts, body_pair, body_triple, atom_u64, axis_to_param,
            detect_loop_setup, detect_back_edge};

const P: u64 = 0xFFFF_FFFF_0000_0001;

/// Compile a nox formula to synthesizable VHDL (combinational).
pub fn compile_to_vhdl<const N: usize>(
    order: &Order<N>,
    formula: NounId,
    num_params: u32,
) -> Result<String, CompileError> {
    let mut e = VhdlEmitter::new(num_params);
    e.emit_formula(order, formula)?;
    let result = e.pop_reg();
    Ok(e.finish(&result, num_params))
}

struct VhdlEmitter {
    signals: String,
    body: String,
    next_signal: u32,
    reg_stack: Vec<String>,
    subject: Vec<String>,
}

impl VhdlEmitter {
    fn new(num_params: u32) -> Self {
        let subject: Vec<String> = (0..num_params).rev()
            .map(|i| format!("p{}", i))
            .collect();
        Self {
            signals: String::with_capacity(4096),
            body: String::with_capacity(4096),
            next_signal: 0,
            reg_stack: Vec::new(),
            subject,
        }
    }

    fn alloc_signal(&mut self) -> String {
        let s = format!("t{}", self.next_signal);
        self.next_signal += 1;
        s
    }

    fn push_reg(&mut self) -> String {
        let s = self.alloc_signal();
        self.reg_stack.push(s.clone());
        s
    }

    fn pop_reg(&mut self) -> String {
        self.reg_stack.pop().unwrap_or_else(|| "t0".into())
    }

    fn declare(&mut self, name: &str, bits: u32) {
        self.signals.push_str(&format!(
            "    signal {} : unsigned({} downto 0);\n", name, bits - 1
        ));
    }

    fn assign(&mut self, line: &str) {
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
        self.declare(&dst, 64);
        self.assign(&format!("{} <= {};", dst, src));
        Ok(())
    }

    fn emit_quote<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let val = atom_u64(order, body)?;
        let dst = self.push_reg();
        self.declare(&dst, 64);
        self.assign(&format!("{} <= to_unsigned({}, 64);", dst, val));
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
        let test_s = self.pop_reg();
        self.emit_formula(order, yes)?;
        let yes_s = self.pop_reg();
        self.emit_formula(order, no)?;
        let no_s = self.pop_reg();
        let dst = self.push_reg();
        self.declare(&dst, 64);
        // nox: 0 = yes (true), nonzero = no (false)
        self.assign(&format!(
            "{} <= {} when {} = to_unsigned(0, 64) else {};",
            dst, yes_s, test_s, no_s
        ));
        Ok(())
    }

    // -- Goldilocks arithmetic ------------------------------------------------

    fn emit_add<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();

        // 65-bit extended sum for overflow detection
        let sum_ext = self.alloc_signal();
        let sum_raw = self.alloc_signal();
        let sum_adj = self.alloc_signal();

        self.declare(&sum_ext, 65);
        self.declare(&sum_raw, 64);
        self.declare(&sum_adj, 64);
        self.declare(&dst, 64);

        self.assign(&format!(
            "{} <= resize({}, 65) + resize({}, 65);",
            sum_ext, ra, rb
        ));
        self.assign(&format!(
            "{} <= {}(63 downto 0);",
            sum_raw, sum_ext
        ));
        // On overflow: add 2^32-1 (P = 2^64 - 2^32 + 1, so 2^64 - P = 2^32 - 1)
        self.assign(&format!(
            "{} <= ({} + to_unsigned({}, 64)) when {}(64) = '1' else {};",
            sum_adj, sum_raw, 0xFFFF_FFFFu64, sum_ext, sum_raw
        ));
        // Final reduction: if sum_adj >= P, subtract P
        self.assign(&format!(
            "{} <= ({} - to_unsigned({}, 64)) when {} >= to_unsigned({}, 64) else {};",
            dst, sum_adj, P, sum_adj, P, sum_adj
        ));
        Ok(())
    }

    fn emit_sub<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();

        let diff = self.alloc_signal();
        let wrap = self.alloc_signal();

        self.declare(&diff, 64);
        self.declare(&wrap, 64);
        self.declare(&dst, 64);

        // If a >= b: result = a - b
        // If a < b:  result = P - b + a  (wraps around in field)
        self.assign(&format!("{} <= {} - {};", diff, ra, rb));
        self.assign(&format!(
            "{} <= to_unsigned({}, 64) - {} + {};",
            wrap, P, rb, ra
        ));
        self.assign(&format!(
            "{} <= {} when {} >= {} else {};",
            dst, diff, ra, rb, wrap
        ));
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
        let prod = self.alloc_signal();
        let lo = self.alloc_signal();
        let hi = self.alloc_signal();

        self.declare(&prod, 128);
        self.declare(&lo, 64);
        self.declare(&hi, 64);

        self.assign(&format!(
            "{} <= resize({}, 128) * resize({}, 128);",
            prod, ra, rb
        ));
        self.assign(&format!("{} <= {}(63 downto 0);", lo, prod));
        self.assign(&format!("{} <= {}(127 downto 64);", hi, prod));

        // Goldilocks reduction: result = lo + hi * (2^32 - 1) mod P
        // hi*(2^32-1) = (hi << 32) - hi
        let hi_shift = self.alloc_signal();
        let hi_term = self.alloc_signal();
        let red_ext = self.alloc_signal();
        let red_raw = self.alloc_signal();
        let red_adj = self.alloc_signal();

        self.declare(&hi_shift, 96);
        self.declare(&hi_term, 96);
        self.declare(&red_ext, 97);
        self.declare(&red_raw, 97);
        self.declare(&red_adj, 64);
        self.declare(&dst, 64);

        self.assign(&format!(
            "{} <= shift_left(resize({}, 96), 32);",
            hi_shift, hi
        ));
        self.assign(&format!(
            "{} <= {} - resize({}, 96);",
            hi_term, hi_shift, hi
        ));
        // Add lo + hi_term
        self.assign(&format!(
            "{} <= resize({}, 97) + resize({}, 97);",
            red_ext, hi_term, lo
        ));
        // First reduction: if high bits set or >= P, subtract P
        self.assign(&format!(
            "{} <= ({} - resize(to_unsigned({}, 64), 97)) when ({}(96 downto 64) /= to_unsigned(0, 33)) or ({}(63 downto 0) >= to_unsigned({}, 64)) else {};",
            red_raw, red_ext, P, red_ext, red_ext, P, red_ext
        ));
        // Second reduction pass
        self.assign(&format!(
            "{} <= ({}(63 downto 0) - to_unsigned({}, 64)) when {}(63 downto 0) >= to_unsigned({}, 64) else {}(63 downto 0);",
            red_adj, red_raw, P, red_raw, P, red_raw
        ));
        self.assign(&format!("{} <= {};", dst, red_adj));
        Ok(())
    }

    fn emit_eq<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.declare(&dst, 64);
        // nox eq: 0 if equal, 1 if not
        self.assign(&format!(
            "{} <= to_unsigned(0, 64) when {} = {} else to_unsigned(1, 64);",
            dst, ra, rb
        ));
        Ok(())
    }

    fn emit_lt<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.declare(&dst, 64);
        // nox lt: 0 if a < b, 1 if a >= b
        self.assign(&format!(
            "{} <= to_unsigned(0, 64) when {} < {} else to_unsigned(1, 64);",
            dst, ra, rb
        ));
        Ok(())
    }

    fn emit_xor<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.declare(&dst, 64);
        self.assign(&format!("{} <= {} xor {};", dst, ra, rb));
        Ok(())
    }

    fn emit_and<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.declare(&dst, 64);
        self.assign(&format!("{} <= {} and {};", dst, ra, rb));
        Ok(())
    }

    fn emit_not<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        self.emit_formula(order, body)?;
        let ra = self.pop_reg();
        let dst = self.push_reg();
        self.declare(&dst, 64);
        self.assign(&format!("{} <= not {};", dst, ra));
        Ok(())
    }

    fn emit_shl<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.declare(&dst, 64);
        // Use lower 6 bits of shift amount (max shift 63)
        self.assign(&format!(
            "{} <= shift_left({}, to_integer({}(5 downto 0)));",
            dst, ra, rb
        ));
        Ok(())
    }

    /// Assemble the complete VHDL design unit.
    fn finish(self, result: &str, num_params: u32) -> String {
        let mut v = String::with_capacity(8192);
        v.push_str("-- nox formula -> synthesizable VHDL (combinational)\n");
        v.push_str("-- Goldilocks field: p = 2^64 - 2^32 + 1\n");
        v.push_str(&format!("-- {} parameter(s), {} signal(s)\n\n", num_params, self.next_signal));

        v.push_str("library IEEE;\nuse IEEE.STD_LOGIC_1164.ALL;\nuse IEEE.NUMERIC_STD.ALL;\n\n");

        v.push_str("entity nox_formula is\n");
        v.push_str("    port (\n");
        for i in 0..num_params {
            v.push_str(&format!(
                "        p{} : in  unsigned(63 downto 0);\n", i
            ));
        }
        v.push_str("        result : out unsigned(63 downto 0)\n");
        v.push_str("    );\n");
        v.push_str("end entity;\n\n");

        v.push_str("architecture behavioral of nox_formula is\n");
        // Signal declarations
        v.push_str(&self.signals);
        v.push_str("begin\n");
        // Concurrent signal assignments
        v.push_str(&self.body);
        // Final assignment
        v.push_str(&format!("    result <= {};\n", result));
        v.push_str("end architecture;\n");
        v
    }
}
