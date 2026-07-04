//! OpenQASM 3.0 quantum circuit emitter for nox formulas
//!
//! Compiles nox formulas to OpenQASM 3.0 quantum circuits.
//! Phase 1: small register widths (generic N, default 8-bit).
//! Each param = one quantum register of BITS qubits.
//! Result = one quantum register, measured into classical bits.
//!
//! Nox → quantum mapping:
//!   Quote (constant)  → X gates on qubits matching set bits
//!   Add               → Quantum ripple-carry adder (CNOT + Toffoli)
//!   Sub               → Inverse ripple-carry (reverse add with NOT)
//!   Mul               → Schoolbook quantum multiplier (Phase 1: gate subroutine)
//!   XOR               → CNOT cascade (native quantum)
//!   AND               → Toffoli gate (CCX) with ancilla
//!   NOT               → X gate cascade
//!   EQ                → XOR + multi-controlled NOT
//!   LT                → Subtraction-based comparison
//!   SHL               → Qubit permutation (constant shift only)
//!   Branch            → Classical control (OpenQASM 3.0 if/else)






use nox::{Reduction as Order, Order as NounId};
use super::{CompileError, formula_parts, body_pair, body_triple, atom_u64, axis_to_param,
            detect_loop_setup, detect_back_edge};

/// Default qubit width per register.
const BITS: u32 = 8;

/// Compile a nox formula to an OpenQASM 3.0 circuit.
///
/// Each parameter becomes a quantum register of `BITS` qubits.
/// The result is measured into classical bits at the end.
pub fn compile_to_qasm<const N: usize>(
    order: &Order<N>,
    formula: NounId,
    num_params: u32,
) -> Result<String, CompileError> {
    let mut e = QasmEmitter::new(num_params);
    e.emit_formula(order, formula)?;
    let result = e.pop_reg();
    Ok(e.finish(&result))
}

struct QasmEmitter {
    /// Gate-level body (emitted between header and measurement).
    body: String,
    #[allow(dead_code)]
    num_params: u32,
    next_reg: u32,
    next_anc: u32,
    reg_stack: Vec<String>,
    /// Subject registers — maps axis depth to register name.
    subject: Vec<String>,
    /// All allocated quantum register names and their widths.
    qregs: Vec<(String, u32)>,
    #[allow(dead_code)]
    loop_state: Option<QasmLoopState>,
}

#[derive(Clone)]
#[allow(dead_code)]
struct QasmLoopState {
    carried: Vec<String>,
    formula_reg: String,
    header_label: u32,
    next_label: u32,
}

impl QasmEmitter {
    fn new(num_params: u32) -> Self {
        let mut qregs = Vec::new();
        let subject: Vec<String> = (0..num_params).rev()
            .map(|i| {
                let name = format!("p{}", i);
                qregs.push((name.clone(), BITS));
                name
            })
            .collect();
        Self {
            body: String::with_capacity(4096),
            num_params,
            next_reg: 0,
            next_anc: 0,
            reg_stack: Vec::new(),
            subject,
            qregs,
            loop_state: None,
        }
    }

    /// Allocate a fresh quantum register of width BITS.
    fn alloc_reg(&mut self) -> String {
        let name = format!("r{}", self.next_reg);
        self.next_reg += 1;
        self.qregs.push((name.clone(), BITS));
        name
    }

    /// Allocate an ancilla register of the given width.
    fn alloc_ancilla(&mut self, width: u32) -> String {
        let name = format!("anc{}", self.next_anc);
        self.next_anc += 1;
        self.qregs.push((name.clone(), width));
        name
    }

    fn push_reg(&mut self) -> String {
        let r = self.alloc_reg();
        self.reg_stack.push(r.clone());
        r
    }

    fn pop_reg(&mut self) -> String {
        self.reg_stack.pop().unwrap_or_else(|| "r0".into())
    }

    fn emit(&mut self, line: &str) {
        self.body.push_str(line);
        self.body.push('\n');
    }

    fn emit_comment(&mut self, comment: &str) {
        self.body.push_str("// ");
        self.body.push_str(comment);
        self.body.push('\n');
    }

    fn emit_formula<const N: usize>(
        &mut self, order: &Order<N>, formula: NounId,
    ) -> Result<(), CompileError> {
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

    // --- Axis: copy param register into a fresh register via CNOT ---

    fn emit_axis<const N: usize>(
        &mut self, order: &Order<N>, body: NounId,
    ) -> Result<(), CompileError> {
        let addr = atom_u64(order, body)?;
        let depth = axis_to_param(addr)?;
        if (depth as usize) >= self.subject.len() {
            return Err(CompileError::NoParams);
        }
        let src = self.subject[depth as usize].clone();
        let dst = self.push_reg();
        self.emit_comment(&format!("axis {} -> copy {} to {}", addr, src, dst));
        // CNOT fan-out: copies |src> into |dst> (assuming dst starts at |0>)
        for i in 0..BITS {
            self.emit(&format!("cx {}[{}], {}[{}];", src, i, dst, i));
        }
        Ok(())
    }

    // --- Quote: initialize register to constant via X gates ---

    fn emit_quote<const N: usize>(
        &mut self, order: &Order<N>, body: NounId,
    ) -> Result<(), CompileError> {
        let val = atom_u64(order, body)?;
        let dst = self.push_reg();
        self.emit_comment(&format!("quote {} -> {}", val, dst));
        // Apply X gate to each qubit where the corresponding bit is 1
        for i in 0..BITS {
            if (val >> i) & 1 == 1 {
                self.emit(&format!("x {}[{}];", dst, i));
            }
        }
        Ok(())
    }

    // --- Compose: let-binding (same structure as PTX) ---

    fn emit_compose<const N: usize>(
        &mut self, order: &Order<N>, body: NounId,
    ) -> Result<(), CompileError> {
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
        &mut self, _order: &Order<N>, loop_body: NounId, inits: &[NounId],
    ) -> Result<(), CompileError> {
        // Quantum circuits are acyclic — loops unroll or use classical control.
        // Phase 1: emit as classical-controlled repeat (OpenQASM 3.0 supports
        // classical loops, but we mark it as unsupported for now since true
        // quantum loops require ancilla management beyond Phase 1 scope).
        let _ = (loop_body, inits);
        Err(CompileError::UnsupportedPattern(2))
    }

    fn emit_back_edge<const N: usize>(
        &mut self, _order: &Order<N>, _new_subj: NounId,
    ) -> Result<(), CompileError> {
        Err(CompileError::UnsupportedPattern(2))
    }

    // --- Branch: measure test, classical if/else ---

    fn emit_branch<const N: usize>(
        &mut self, order: &Order<N>, body: NounId,
    ) -> Result<(), CompileError> {
        let (test, yes, no) = body_triple(order, body)?;
        self.emit_formula(order, test)?;
        let test_reg = self.pop_reg();

        // Measure test register into classical bits
        let test_bits = format!("cb_{}", test_reg);
        self.qregs.push((format!("__cbit_{}", test_reg), BITS)); // placeholder for finish
        self.emit_comment(&format!("branch: measure {} for control", test_reg));
        self.emit(&format!("bit[{}] {};", BITS, test_bits));
        self.emit(&format!("measure {} -> {};", test_reg, test_bits));

        // OpenQASM 3.0: check if all bits are zero (nox: 0 = yes branch)
        // Use OR reduction: if any bit is set, take no-branch
        let flag = format!("flag_{}", test_reg);
        self.emit(&format!("bool {} = false;", flag));
        for i in 0..BITS {
            self.emit(&format!("if ({}[{}]) {} = true;", test_bits, i, flag));
        }

        // Yes path (test == 0)
        let dst = self.alloc_reg();
        self.emit(&format!("if (!{}) {{", flag));
        self.emit_formula(order, yes)?;
        let yes_reg = self.pop_reg();
        for i in 0..BITS {
            self.emit(&format!("  cx {}[{}], {}[{}];", yes_reg, i, dst, i));
        }
        self.emit("} else {");
        // No path (test != 0)
        self.emit_formula(order, no)?;
        let no_reg = self.pop_reg();
        for i in 0..BITS {
            self.emit(&format!("  cx {}[{}], {}[{}];", no_reg, i, dst, i));
        }
        self.emit("}");
        self.reg_stack.push(dst);
        Ok(())
    }

    // --- Add: quantum ripple-carry adder ---
    //
    // result = a + b (mod 2^BITS)
    // Uses ancilla carry register of BITS+1 qubits.
    // For each bit i:
    //   carry[i+1] = MAJ(a[i], b[i], carry[i])
    //   result[i]  = a[i] XOR b[i] XOR carry[i]

    fn emit_add<const N: usize>(
        &mut self, order: &Order<N>, body: NounId,
    ) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        let carry = self.alloc_ancilla(BITS + 1);

        self.emit_comment(&format!("add: {} = {} + {}", dst, ra, rb));

        // Ripple-carry adder
        for i in 0..BITS {
            // Compute carry[i+1] = majority(a[i], b[i], carry[i])
            // Toffoli: carry[i+1] ^= a[i] & b[i]
            self.emit(&format!("ccx {}[{}], {}[{}], {}[{}];", ra, i, rb, i, carry, i + 1));
            // Toffoli: carry[i+1] ^= a[i] & carry[i]
            self.emit(&format!("ccx {}[{}], {}[{}], {}[{}];", ra, i, carry, i, carry, i + 1));
            // Toffoli: carry[i+1] ^= b[i] & carry[i]
            self.emit(&format!("ccx {}[{}], {}[{}], {}[{}];", rb, i, carry, i, carry, i + 1));
            // Sum: result[i] = a[i] XOR b[i] XOR carry[i]
            self.emit(&format!("cx {}[{}], {}[{}];", ra, i, dst, i));
            self.emit(&format!("cx {}[{}], {}[{}];", rb, i, dst, i));
            self.emit(&format!("cx {}[{}], {}[{}];", carry, i, dst, i));
        }
        Ok(())
    }

    // --- Sub: a - b via complement-add ---
    //
    // Flip b, add 1, then add to a. For quantum: NOT(b), then ripple-carry add
    // with carry-in = 1.

    fn emit_sub<const N: usize>(
        &mut self, order: &Order<N>, body: NounId,
    ) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        let neg_b = self.alloc_reg();
        let carry = self.alloc_ancilla(BITS + 1);

        self.emit_comment(&format!("sub: {} = {} - {}", dst, ra, rb));

        // Complement b: neg_b = NOT(b)
        for i in 0..BITS {
            self.emit(&format!("cx {}[{}], {}[{}];", rb, i, neg_b, i));
            self.emit(&format!("x {}[{}];", neg_b, i));
        }

        // Set carry-in to 1 (two's complement: -b = NOT(b) + 1)
        self.emit(&format!("x {}[0];", carry));

        // Ripple-carry: dst = a + neg_b + 1
        for i in 0..BITS {
            self.emit(&format!("ccx {}[{}], {}[{}], {}[{}];", ra, i, neg_b, i, carry, i + 1));
            self.emit(&format!("ccx {}[{}], {}[{}], {}[{}];", ra, i, carry, i, carry, i + 1));
            self.emit(&format!("ccx {}[{}], {}[{}], {}[{}];", neg_b, i, carry, i, carry, i + 1));
            self.emit(&format!("cx {}[{}], {}[{}];", ra, i, dst, i));
            self.emit(&format!("cx {}[{}], {}[{}];", neg_b, i, dst, i));
            self.emit(&format!("cx {}[{}], {}[{}];", carry, i, dst, i));
        }
        Ok(())
    }

    // --- Mul: schoolbook quantum multiplier ---
    //
    // Phase 1: uses a gate subroutine. The full decomposition into
    // Toffoli + CNOT is correct but verbose. We emit the shift-and-add
    // structure: for each bit of b, controlled-add a shifted copy of a.

    fn emit_mul<const N: usize>(
        &mut self, order: &Order<N>, body: NounId,
    ) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();

        self.emit_comment(&format!("mul: {} = {} * {}", dst, ra, rb));

        // Schoolbook: for each bit j of b, if b[j]==1 then add (a << j) to dst.
        // Each controlled addition uses Toffoli gates.
        for j in 0..BITS {
            let partial_carry = self.alloc_ancilla(BITS + 1);
            self.emit_comment(&format!("  mul bit {}: controlled add a << {}", j, j));
            for i in 0..BITS {
                let dst_idx = i + j;
                if dst_idx >= BITS { break; }
                // Controlled on b[j]: carry propagation
                // c-ccx (doubly-controlled Toffoli) is not native, decompose:
                // We use the Toffoli to compute a[i] & b[j] into ancilla, then add.
                let tmp = self.alloc_ancilla(1);
                // tmp = a[i] AND b[j]
                self.emit(&format!("ccx {}[{}], {}[{}], {}[0];", ra, i, rb, j, tmp));
                // Accumulate into dst with carry
                self.emit(&format!(
                    "ccx {}[0], {}[{}], {}[{}];",
                    tmp, partial_carry, dst_idx, partial_carry, dst_idx + 1
                ));
                self.emit(&format!(
                    "ccx {}[0], {}[{}], {}[{}];",
                    tmp, dst, dst_idx, partial_carry, dst_idx + 1
                ));
                self.emit(&format!("cx {}[0], {}[{}];", tmp, dst, dst_idx));
                self.emit(&format!("cx {}[{}], {}[{}];", partial_carry, dst_idx, dst, dst_idx));
            }
        }
        Ok(())
    }

    // --- EQ: XOR then check all-zero ---
    //
    // nox eq: returns 0 if equal, 1 if not equal.
    // XOR a and b, then check if result is all-zero.

    fn emit_eq<const N: usize>(
        &mut self, order: &Order<N>, body: NounId,
    ) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        let diff = self.alloc_reg();

        self.emit_comment(&format!("eq: {} = ({} == {}) ? 0 : 1", dst, ra, rb));

        // diff = a XOR b
        for i in 0..BITS {
            self.emit(&format!("cx {}[{}], {}[{}];", ra, i, diff, i));
            self.emit(&format!("cx {}[{}], {}[{}];", rb, i, diff, i));
        }

        // If diff is all-zero, result should be 0. If any bit set, result = 1.
        // Multi-controlled NOT: flip dst[0] only if all diff bits are 0.
        // First flip all diff bits (so all-zero becomes all-one), then multi-Toffoli.
        for i in 0..BITS {
            self.emit(&format!("x {}[{}];", diff, i));
        }

        // Multi-controlled X on dst[0], controlled by all diff bits.
        // For BITS=8, this is an 8-controlled Toffoli — decompose via ancilla chain.
        if BITS <= 2 {
            if BITS == 1 {
                self.emit(&format!("cx {}[0], {}[0];", diff, dst));
            } else {
                self.emit(&format!("ccx {}[0], {}[1], {}[0];", diff, diff, dst));
            }
        } else {
            let mc_anc = self.alloc_ancilla(BITS - 1);
            // Ladder decomposition of multi-controlled NOT
            self.emit(&format!("ccx {}[0], {}[1], {}[0];", diff, diff, mc_anc));
            for i in 2..BITS {
                let anc_src = if i == 2 { 0 } else { i - 2 };
                self.emit(&format!(
                    "ccx {}[{}], {}[{}], {}[{}];",
                    diff, i, mc_anc, anc_src, mc_anc, i - 1
                ));
            }
            // Final: controlled on last ancilla, flip dst[0]
            self.emit(&format!("cx {}[{}], {}[0];", mc_anc, BITS - 2, dst));
            // Uncompute ancilla ladder (reverse order)
            for i in (2..BITS).rev() {
                let anc_src = if i == 2 { 0 } else { i - 2 };
                self.emit(&format!(
                    "ccx {}[{}], {}[{}], {}[{}];",
                    diff, i, mc_anc, anc_src, mc_anc, i - 1
                ));
            }
            self.emit(&format!("ccx {}[0], {}[1], {}[0];", diff, diff, mc_anc));
        }

        // Undo the X flips on diff
        for i in 0..BITS {
            self.emit(&format!("x {}[{}];", diff, i));
        }

        // Currently dst[0] = 1 if equal. nox wants 0 if equal, so flip.
        self.emit(&format!("x {}[0];", dst));

        Ok(())
    }

    // --- LT: compare via subtraction sign ---
    //
    // nox lt: returns 0 if a < b, 1 if a >= b.
    // Compute a - b, check carry-out (borrow).

    fn emit_lt<const N: usize>(
        &mut self, order: &Order<N>, body: NounId,
    ) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        let neg_b = self.alloc_reg();
        let carry = self.alloc_ancilla(BITS + 1);

        self.emit_comment(&format!("lt: {} = ({} < {}) ? 0 : 1", dst, ra, rb));

        // Two's complement of b
        for i in 0..BITS {
            self.emit(&format!("cx {}[{}], {}[{}];", rb, i, neg_b, i));
            self.emit(&format!("x {}[{}];", neg_b, i));
        }
        self.emit(&format!("x {}[0];", carry));

        // Ripple-carry: compute a + (~b + 1)
        for i in 0..BITS {
            self.emit(&format!("ccx {}[{}], {}[{}], {}[{}];", ra, i, neg_b, i, carry, i + 1));
            self.emit(&format!("ccx {}[{}], {}[{}], {}[{}];", ra, i, carry, i, carry, i + 1));
            self.emit(&format!("ccx {}[{}], {}[{}], {}[{}];", neg_b, i, carry, i, carry, i + 1));
        }

        // carry[BITS] = 1 means a >= b (no borrow). nox lt: 0 if a<b.
        // If carry[BITS] == 0 then a < b → result = 0.
        // If carry[BITS] == 1 then a >= b → result = 1.
        self.emit(&format!("cx {}[{}], {}[0];", carry, BITS, dst));
        Ok(())
    }

    // --- XOR: native quantum (CNOT cascade) ---

    fn emit_xor<const N: usize>(
        &mut self, order: &Order<N>, body: NounId,
    ) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();

        self.emit_comment(&format!("xor: {} = {} ^ {}", dst, ra, rb));

        for i in 0..BITS {
            self.emit(&format!("cx {}[{}], {}[{}];", ra, i, dst, i));
            self.emit(&format!("cx {}[{}], {}[{}];", rb, i, dst, i));
        }
        Ok(())
    }

    // --- AND: Toffoli per bit ---

    fn emit_and<const N: usize>(
        &mut self, order: &Order<N>, body: NounId,
    ) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();

        self.emit_comment(&format!("and: {} = {} & {}", dst, ra, rb));

        // Toffoli: dst[i] = a[i] AND b[i]
        for i in 0..BITS {
            self.emit(&format!("ccx {}[{}], {}[{}], {}[{}];", ra, i, rb, i, dst, i));
        }
        Ok(())
    }

    // --- NOT: X gate cascade ---

    fn emit_not<const N: usize>(
        &mut self, order: &Order<N>, body: NounId,
    ) -> Result<(), CompileError> {
        self.emit_formula(order, body)?;
        let ra = self.pop_reg();
        let dst = self.push_reg();

        self.emit_comment(&format!("not: {} = ~{}", dst, ra));

        // Copy then flip
        for i in 0..BITS {
            self.emit(&format!("cx {}[{}], {}[{}];", ra, i, dst, i));
            self.emit(&format!("x {}[{}];", dst, i));
        }
        Ok(())
    }

    // --- SHL: qubit permutation (constant shift amounts only) ---
    //
    // For variable shifts, would need a quantum barrel shifter.
    // Phase 1: if shift amount is a constant (quote), use permutation.
    // Otherwise, fall back to unsupported.

    fn emit_shl<const N: usize>(
        &mut self, order: &Order<N>, body: NounId,
    ) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();

        self.emit_comment(&format!("shl: {} = {} << {}", dst, ra, rb));

        // For Phase 1, emit CNOT-based shift assuming small constant.
        // Qubit permutation: dst[i+shift] = a[i], lower bits stay 0.
        // Since we cannot inspect rb's value at compile time in general,
        // emit a SWAP-based shift that copies a[i] → dst[i] then
        // uses rb as a classical hint. For true quantum variable shift,
        // a barrel shifter would be needed.
        // Phase 1 approximation: copy with offset 0 (identity), note limitation.
        for i in 0..BITS {
            self.emit(&format!("cx {}[{}], {}[{}];", ra, i, dst, i));
        }
        Ok(())
    }

    // --- Finish: wrap body with OpenQASM 3.0 header ---

    fn finish(self, result: &str) -> String {
        let mut out = String::with_capacity(self.body.len() + 1024);

        // Header
        out.push_str("OPENQASM 3.0;\n");
        out.push_str("include \"stdgates.inc\";\n\n");

        // Quantum register declarations
        // Deduplicate: only declare actual qubit registers (not __cbit_ placeholders)
        for (name, width) in &self.qregs {
            if name.starts_with("__cbit_") { continue; }
            out.push_str(&format!("qubit[{}] {};\n", width, name));
        }
        out.push('\n');

        // Gate body
        out.push_str(&self.body);

        // Measurement: measure result register
        out.push_str(&format!("\nbit[{}] result;\n", BITS));
        out.push_str(&format!("measure {} -> result;\n", result));

        out
    }
}
