//! QIR (Quantum Intermediate Representation) text emitter for nox formulas
//!
//! Compiles nox formulas to QIR — LLVM IR with quantum extensions.
//! Microsoft Azure Quantum standard, QIR base profile.
//!
//! Each nox param → 8 qubits (8-bit quantum register).
//! Arithmetic operations map to external quantum functions
//! (@quantum_add, @quantum_mul, etc.) that the runtime expands
//! into gate sequences.
//!
//! Phase 1: emit valid LLVM IR with quantum intrinsic calls.
//! Gate decompositions for arithmetic defined as external declarations.
//! The runtime fills in the actual gate sequences.






use nox::{Reduction as Order, Order as NounId};
use super::{CompileError, formula_parts, body_pair, body_triple, atom_u64, axis_to_param,
            detect_loop_setup, detect_back_edge};

const QUBITS_PER_REG: u32 = 8;

/// Compile a nox formula to QIR text (.ll format).
pub fn compile_to_qir<const N: usize>(
    order: &Order<N>,
    formula: NounId,
    num_params: u32,
) -> Result<String, CompileError> {
    let mut e = QirEmitter::new(num_params);
    e.emit_formula(order, formula)?;
    let result = e.pop_reg();
    Ok(e.finish(&result))
}

struct QirEmitter {
    body: String,
    num_params: u32,
    next_reg: u32,
    next_label: u32,
    reg_stack: Vec<String>,
    subject: Vec<String>,
    loop_state: Option<QirLoopState>,
    /// Total qubits allocated (params + intermediates).
    next_qubit: u32,
}

#[derive(Clone)]
struct QirLoopState {
    carried: Vec<String>,
    formula_reg: String,
    header_label: String,
}

impl QirEmitter {
    fn new(num_params: u32) -> Self {
        let subject: Vec<String> = (0..num_params).rev()
            .map(|i| format!("%p{}", i))
            .collect();
        Self {
            body: String::with_capacity(4096),
            num_params,
            next_reg: 0,
            next_label: 0,
            reg_stack: Vec::new(),
            subject,
            loop_state: None,
            next_qubit: num_params * QUBITS_PER_REG,
        }
    }

    fn alloc_reg(&mut self) -> String {
        let r = format!("%r{}", self.next_reg);
        self.next_reg += 1;
        r
    }

    fn alloc_label(&mut self) -> String {
        let l = format!("L{}", self.next_label);
        self.next_label += 1;
        l
    }

    fn alloc_qubit_array(&mut self) -> u32 {
        let base = self.next_qubit;
        self.next_qubit += QUBITS_PER_REG;
        base
    }

    fn push_reg(&mut self) -> String {
        let r = self.alloc_reg();
        self.reg_stack.push(r.clone());
        r
    }

    fn pop_reg(&mut self) -> String {
        self.reg_stack.pop().unwrap_or_else(|| "%r0".to_string())
    }

    fn emit(&mut self, line: &str) {
        self.body.push_str("  ");
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
        self.emit(&format!("{} = add i64 {}, 0", dst, src));
        Ok(())
    }

    fn emit_quote<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let val = atom_u64(order, body)?;
        let dst = self.push_reg();
        self.emit(&format!("{} = add i64 0, {}", dst, val));
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
        self.emit(&format!("{} = add i64 0, 0", formula_reg));

        let mut carried = Vec::new();
        for &init in inits.iter() {
            self.emit_formula(order, init)?;
            let val = self.pop_reg();
            let cr = self.alloc_reg();
            self.emit(&format!("{} = add i64 {}, 0", cr, val));
            carried.push(cr);
        }

        let saved = self.subject.clone();
        for cr in carried.iter() {
            self.subject.insert(0, cr.clone());
        }
        self.subject.insert(0, formula_reg.clone());

        let header = self.alloc_label();
        let prev = self.loop_state.take();
        self.loop_state = Some(QirLoopState {
            carried: carried.clone(),
            formula_reg: formula_reg.clone(),
            header_label: header.clone(),
        });

        self.emit(&format!("br label %{}", header));
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
        // In LLVM IR we cannot reassign SSA values, so we use alloca-like
        // patterns. For Phase 1 loops, emit a branch back to header.
        // The carried values are referenced by name in the loop body.
        self.emit(&format!("br label %{}", ls.header_label));
        let _ = self.push_reg(); // dummy result
        Ok(())
    }

    fn emit_branch<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (test, yes, no) = body_triple(order, body)?;
        self.emit_formula(order, test)?;
        let test_r = self.pop_reg();
        let lbl_yes = self.alloc_label();
        let lbl_no = self.alloc_label();
        let lbl_end = self.alloc_label();

        // nox: 0=yes, nonzero=no
        let cond = self.alloc_reg();
        self.emit(&format!("{} = icmp eq i64 {}, 0", cond, test_r));
        self.emit(&format!("br i1 {}, label %{}, label %{}", cond, lbl_yes, lbl_no));

        // yes path (test==0)
        self.emit_label(&lbl_yes);
        self.emit_formula(order, yes)?;
        let yes_r = self.pop_reg();
        self.emit(&format!("br label %{}", lbl_end));

        // no path
        self.emit_label(&lbl_no);
        self.emit_formula(order, no)?;
        let no_r = self.pop_reg();
        self.emit(&format!("br label %{}", lbl_end));

        // merge
        self.emit_label(&lbl_end);
        let dst = self.push_reg();
        self.emit(&format!("{} = phi i64 [ {}, %{} ], [ {}, %{} ]",
            dst, yes_r, lbl_yes, no_r, lbl_no));
        Ok(())
    }

    // --- Arithmetic: delegate to external quantum functions ---
    // Each operation calls an external function that operates on
    // classical i64 values. The quantum runtime maps these to
    // reversible gate circuits on qubit registers.

    fn emit_add<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("{} = call i64 @quantum_add(i64 {}, i64 {})", dst, ra, rb));
        Ok(())
    }

    fn emit_sub<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("{} = call i64 @quantum_sub(i64 {}, i64 {})", dst, ra, rb));
        Ok(())
    }

    fn emit_mul<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("{} = call i64 @quantum_mul(i64 {}, i64 {})", dst, ra, rb));
        Ok(())
    }

    fn emit_eq<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("{} = call i64 @quantum_eq(i64 {}, i64 {})", dst, ra, rb));
        Ok(())
    }

    fn emit_lt<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("{} = call i64 @quantum_lt(i64 {}, i64 {})", dst, ra, rb));
        Ok(())
    }

    fn emit_xor<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("{} = xor i64 {}, {}", dst, ra, rb));
        Ok(())
    }

    fn emit_and<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("{} = and i64 {}, {}", dst, ra, rb));
        Ok(())
    }

    fn emit_not<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        self.emit_formula(order, body)?;
        let ra = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("{} = xor i64 {}, 255", dst, ra));
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
        self.emit(&format!("{} = shl i64 {}, {}", tmp, ra, rb));
        self.emit(&format!("{} = and i64 {}, 255", dst, tmp));
        Ok(())
    }

    /// Assemble the final QIR module.
    fn finish(self, result: &str) -> String {
        let mut qir = String::with_capacity(8192);

        // Module header
        qir.push_str("; QIR module generated by trident\n");
        qir.push_str("; Target: quantum (QIR base profile)\n\n");

        // Opaque types
        qir.push_str("%Qubit = type opaque\n");
        qir.push_str("%Result = type opaque\n\n");

        // QIR runtime intrinsics
        qir.push_str("; QIR runtime — qubit management\n");
        qir.push_str("declare %Qubit* @__quantum__rt__qubit_allocate()\n");
        qir.push_str("declare void @__quantum__rt__qubit_release(%Qubit*)\n\n");

        // QIR gate intrinsics
        qir.push_str("; QIR gate intrinsics\n");
        qir.push_str("declare void @__quantum__qis__h__body(%Qubit*)\n");
        qir.push_str("declare void @__quantum__qis__x__body(%Qubit*)\n");
        qir.push_str("declare void @__quantum__qis__cnot__body(%Qubit*, %Qubit*)\n");
        qir.push_str("declare void @__quantum__qis__rz__body(double, %Qubit*)\n");
        qir.push_str("declare void @__quantum__qis__mz__body(%Qubit*, %Result*)\n\n");

        // Arithmetic stubs — external quantum functions
        qir.push_str("; Quantum arithmetic (gate decompositions provided by runtime)\n");
        qir.push_str("declare i64 @quantum_add(i64, i64)\n");
        qir.push_str("declare i64 @quantum_sub(i64, i64)\n");
        qir.push_str("declare i64 @quantum_mul(i64, i64)\n");
        qir.push_str("declare i64 @quantum_eq(i64, i64)\n");
        qir.push_str("declare i64 @quantum_lt(i64, i64)\n\n");

        // Main function
        qir.push_str("define i64 @main(");
        for i in 0..self.num_params {
            if i > 0 { qir.push_str(", "); }
            qir.push_str(&format!("i64 %p{}", i));
        }
        qir.push_str(") {\n");
        qir.push_str("entry:\n");

        // Allocate qubits for each param register
        for i in 0..self.num_params {
            for q in 0..QUBITS_PER_REG {
                let idx = i * QUBITS_PER_REG + q;
                qir.push_str(&format!("  %q{} = call %Qubit* @__quantum__rt__qubit_allocate()\n", idx));
            }
        }
        if self.num_params > 0 {
            qir.push('\n');
        }

        // Body
        qir.push_str(&self.body);

        // Release qubits
        qir.push('\n');
        for i in 0..self.num_params {
            for q in 0..QUBITS_PER_REG {
                let idx = i * QUBITS_PER_REG + q;
                qir.push_str(&format!("  call void @__quantum__rt__qubit_release(%Qubit* %q{})\n", idx));
            }
        }

        // Return result
        qir.push_str(&format!("\n  ret i64 {}\n", result));
        qir.push_str("}\n");
        qir
    }
}
