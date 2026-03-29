//! WASM binary emitter for nox formulas
//!
//! Compiles a nox formula into a standalone .wasm module.
//! Phase 1: atom-only — function params are i64 Goldilocks elements,
//! returns a single i64.
//!
//! Module layout:
//!   func 0: goldilocks_mul(a: i64, b: i64) -> i64  (helper)
//!   func 1: main(params...) -> i64                  (formula)
//!
//! Export: "main" → func 1




use nox::noun::{Order, NounId};
use super::{CompileError, formula_parts, body_pair, body_triple, atom_u64, axis_to_param,
            detect_loop_setup, detect_back_edge};

const P: u64 = 0xFFFF_FFFF_0000_0001;

// ── WASM opcodes ─────────────────────────────────────────────────

const OP_UNREACHABLE: u8 = 0x00;
const OP_IF: u8 = 0x04;
const OP_ELSE: u8 = 0x05;
const OP_END: u8 = 0x0B;
const OP_CALL: u8 = 0x10;
const OP_LOCAL_GET: u8 = 0x20;
const OP_LOCAL_SET: u8 = 0x21;
const OP_LOCAL_TEE: u8 = 0x22;
const OP_I64_CONST: u8 = 0x42;
const OP_I64_EQZ: u8 = 0x50;
const OP_I64_EQ: u8 = 0x51;
const OP_I64_LT_U: u8 = 0x53;
const OP_I64_GE_U: u8 = 0x57;
const OP_I64_ADD: u8 = 0x7C;
const OP_I64_SUB: u8 = 0x7D;
const OP_I64_MUL: u8 = 0x7E;
const OP_I64_AND: u8 = 0x83;
const OP_I64_XOR: u8 = 0x85;
const OP_I64_SHL: u8 = 0x86;
const OP_I64_SHR_U: u8 = 0x88;
const OP_I64_EXTEND_I32_U: u8 = 0xAD;

const SEC_TYPE: u8 = 1;
const SEC_FUNCTION: u8 = 3;
const SEC_EXPORT: u8 = 7;
const SEC_CODE: u8 = 10;

const TYPE_I64: u8 = 0x7E;
const TYPE_FUNC: u8 = 0x60;

/// Compile a nox formula into WASM binary.
pub fn compile_to_wasm<const N: usize>(
    order: &Order<N>,
    formula: NounId,
    num_params: u32,
) -> Result<Vec<u8>, CompileError> {
    let mut emitter = FormulaEmitter::new(num_params);
    emitter.emit_formula(order, formula)?;
    Ok(build_module(num_params, emitter))
}

// ── formula emitter ──────────────────────────────────────────────

#[derive(Clone)]
struct LoopState {
    carried: Vec<u32>,
    formula_local: u32,
}

struct FormulaEmitter {
    num_params: u32,
    num_locals: u32,
    code: Vec<u8>,
    /// Subject model: maps depth index → WASM local index.
    subject: Vec<u32>,
    /// Current loop state (for back-edge emission)
    loop_locals: Option<LoopState>,
}

impl FormulaEmitter {
    fn new(num_params: u32) -> Self {
        // Initial subject: params in reverse order (last param = head = depth 0)
        let subject = (0..num_params).rev().collect();
        Self { num_params, num_locals: 0, code: Vec::with_capacity(256), subject, loop_locals: None }
    }

    fn alloc_local(&mut self) -> u32 {
        let idx = self.num_params + self.num_locals;
        self.num_locals += 1;
        idx
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
        if (depth as usize) >= self.subject.len() {
            return Err(CompileError::NoParams);
        }
        let local_idx = self.subject[depth as usize];
        self.code.push(OP_LOCAL_GET);
        leb128_u32(&mut self.code, local_idx);
        Ok(())
    }

    /// Pattern 2: compose — detect let-binding or loop pattern.
    fn emit_compose<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        // Check for loop setup: [2 [init_cons [1 loop_body]]]
        if let Some((loop_body, inits)) = detect_loop_setup(order, body) {
            return self.emit_loop(order, loop_body, &inits);
        }

        // Check for back-edge: [2 [new_subj [0 N]]]
        if let Some((new_subj, _axis)) = detect_back_edge(order, body) {
            return self.emit_back_edge(order, new_subj);
        }

        // Let-binding: [2 [[3 [value [0 1]]] [1 body]]]
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
        let local = self.alloc_local();
        self.code.push(OP_LOCAL_SET);
        leb128_u32(&mut self.code, local);
        self.subject.insert(0, local);
        let result = self.emit_formula(order, body_formula);
        self.subject.remove(0);
        result
    }

    /// Emit loop: allocate locals for carried state, emit WASM loop block.
    fn emit_loop<const N: usize>(
        &mut self, order: &Order<N>, loop_body: NounId, inits: &[NounId],
    ) -> Result<(), CompileError> {
        let num_carried = inits.len() as u32;

        // Allocate locals for carried state + formula slot
        let formula_local = self.alloc_local(); // slot 0 (formula — unused in native, just for subject model)
        let mut carried_locals = Vec::new();
        for _ in 0..num_carried {
            carried_locals.push(self.alloc_local());
        }

        // Initialize carried locals from init formulas
        for (i, &init) in inits.iter().enumerate() {
            self.emit_formula(order, init)?;
            self.code.push(OP_LOCAL_SET);
            leb128_u32(&mut self.code, carried_locals[i]);
        }
        // Initialize formula_local to 0 (placeholder, not used in native)
        self.code.push(OP_I64_CONST);
        leb128_i64(&mut self.code, 0);
        self.code.push(OP_LOCAL_SET);
        leb128_u32(&mut self.code, formula_local);

        // Build loop subject: [formula [carried_last [... [carried_first [params...]]]]]
        let saved_subject = self.subject.clone();
        // Push carried locals onto subject (first carried = deepest)
        for &cl in carried_locals.iter() {
            self.subject.insert(0, cl);
        }
        // Push formula slot at depth 0
        self.subject.insert(0, formula_local);

        // Save loop state for back-edge
        let prev_loop = self.loop_locals.take();
        self.loop_locals = Some(LoopState {
            carried: carried_locals.clone(),
            formula_local,
        });

        // Emit WASM loop block: loop { body; br 0; } or exit
        // block $exit {
        //   loop $loop {
        //     ... body (branch exits via br $exit, continue via br $loop) ...
        //   }
        // }
        self.code.push(0x02); // block
        self.code.push(TYPE_I64); // block returns i64
        self.code.push(0x03); // loop
        self.code.push(0x40); // loop has no result (void, we break from block with result)

        // Compile loop body
        self.emit_formula(order, loop_body)?;

        // If we reach here without a br, the loop body fell through = exit
        // The value on stack is the result
        self.code.push(0x0C); // br
        leb128_u32(&mut self.code, 1); // br 1 = exit from outer block with value

        self.code.push(OP_END); // end loop
        self.code.push(OP_END); // end block

        // Restore
        self.loop_locals = prev_loop;
        self.subject = saved_subject;
        Ok(())
    }

    /// Back-edge: update carried locals, br to loop header.
    fn emit_back_edge<const N: usize>(
        &mut self, order: &Order<N>, new_subj_formula: NounId,
    ) -> Result<(), CompileError> {
        let ls = self.loop_locals.as_ref()
            .ok_or(CompileError::UnsupportedPattern(2))?
            .clone();

        // new_subj_formula is a cons chain: [3 [formula_ref [3 [c0 [3 [c1 ... params]]]]]]
        // We need to extract and compile each carried value, then store back.
        // Walk the cons chain, skip the formula slot (first cons), extract carried values.
        let (tag, cons_body) = formula_parts(order, new_subj_formula)?;
        if tag != 3 { return Err(CompileError::UnsupportedPattern(2)); }
        let (_formula_ref, rest) = body_pair(order, cons_body)?;
        // rest is cons chain of carried values
        let mut cur = rest;
        for (i, &local) in ls.carried.iter().enumerate() {
            let (tag, cb) = formula_parts(order, cur)?;
            if tag != 3 {
                // Last item might not be cons — might be the params tail
                break;
            }
            let (val_formula, tail) = body_pair(order, cb)?;
            // Compile new value for this carried local
            self.emit_formula(order, val_formula)?;
            self.code.push(OP_LOCAL_SET);
            leb128_u32(&mut self.code, local);
            cur = tail;
        }

        // br to loop header (innermost loop = index 0)
        self.code.push(0x0C); // br
        leb128_u32(&mut self.code, 0); // br 0 = continue loop

        // Need a dummy value on stack for type checker (unreachable after br)
        self.code.push(OP_UNREACHABLE);
        Ok(())
    }

    fn emit_quote<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let val = atom_u64(order, body)?;
        self.code.push(OP_I64_CONST);
        leb128_i64(&mut self.code, val as i64);
        Ok(())
    }

    fn emit_branch<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (test, yes, no) = body_triple(order, body)?;
        self.emit_formula(order, test)?;
        // nox: 0→yes, nonzero→no
        self.code.push(OP_I64_EQZ);
        self.code.push(OP_IF);
        self.code.push(TYPE_I64);
        self.emit_formula(order, yes)?;
        self.code.push(OP_ELSE);
        self.emit_formula(order, no)?;
        self.code.push(OP_END);
        Ok(())
    }

    fn emit_add<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        let la = self.alloc_local();
        let lb = self.alloc_local();
        let ls = self.alloc_local();

        self.emit_formula(order, a)?;
        self.code.push(OP_LOCAL_SET); leb128_u32(&mut self.code, la);
        self.emit_formula(order, b)?;
        self.code.push(OP_LOCAL_SET); leb128_u32(&mut self.code, lb);

        // sum = a + b
        self.code.push(OP_LOCAL_GET); leb128_u32(&mut self.code, la);
        self.code.push(OP_LOCAL_GET); leb128_u32(&mut self.code, lb);
        self.code.push(OP_I64_ADD);
        self.code.push(OP_LOCAL_TEE); leb128_u32(&mut self.code, ls);

        // overflow? sum < a
        self.code.push(OP_LOCAL_GET); leb128_u32(&mut self.code, la);
        self.code.push(OP_I64_LT_U);
        self.code.push(OP_IF); self.code.push(TYPE_I64);
        {
            // overflow: sum + 0xFFFFFFFF
            self.code.push(OP_LOCAL_GET); leb128_u32(&mut self.code, ls);
            self.code.push(OP_I64_CONST); leb128_i64(&mut self.code, 0xFFFF_FFFF);
            self.code.push(OP_I64_ADD);
            self.code.push(OP_LOCAL_TEE); leb128_u32(&mut self.code, ls);
            self.emit_reduce(ls);
        }
        self.code.push(OP_ELSE);
        {
            // no overflow: reduce if >= p
            self.code.push(OP_LOCAL_GET); leb128_u32(&mut self.code, ls);
            self.emit_reduce(ls);
        }
        self.code.push(OP_END);
        Ok(())
    }

    fn emit_sub<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        let la = self.alloc_local();
        let lb = self.alloc_local();

        self.emit_formula(order, a)?;
        self.code.push(OP_LOCAL_SET); leb128_u32(&mut self.code, la);
        self.emit_formula(order, b)?;
        self.code.push(OP_LOCAL_SET); leb128_u32(&mut self.code, lb);

        self.code.push(OP_LOCAL_GET); leb128_u32(&mut self.code, la);
        self.code.push(OP_LOCAL_GET); leb128_u32(&mut self.code, lb);
        self.code.push(OP_I64_GE_U);
        self.code.push(OP_IF); self.code.push(TYPE_I64);
        {
            self.code.push(OP_LOCAL_GET); leb128_u32(&mut self.code, la);
            self.code.push(OP_LOCAL_GET); leb128_u32(&mut self.code, lb);
            self.code.push(OP_I64_SUB);
        }
        self.code.push(OP_ELSE);
        {
            // p - b + a
            self.code.push(OP_I64_CONST); leb128_i64(&mut self.code, P as i64);
            self.code.push(OP_LOCAL_GET); leb128_u32(&mut self.code, lb);
            self.code.push(OP_I64_SUB);
            self.code.push(OP_LOCAL_GET); leb128_u32(&mut self.code, la);
            self.code.push(OP_I64_ADD);
        }
        self.code.push(OP_END);
        Ok(())
    }

    fn emit_mul<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        // call goldilocks_mul (function index 0)
        self.code.push(OP_CALL);
        leb128_u32(&mut self.code, 0);
        Ok(())
    }

    fn emit_eq<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        self.code.push(OP_I64_EQ);
        self.code.push(OP_I64_EXTEND_I32_U);
        // nox: 0=equal, 1=not equal. WASM i64.eq: 1=equal, 0=not. Flip.
        self.code.push(OP_I64_CONST); leb128_i64(&mut self.code, 1);
        self.code.push(OP_I64_XOR);
        Ok(())
    }

    fn emit_lt<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        self.code.push(OP_I64_LT_U);
        self.code.push(OP_I64_EXTEND_I32_U);
        // nox: 0=true(a<b), 1=false. WASM: 1=true, 0=false. Flip.
        self.code.push(OP_I64_CONST); leb128_i64(&mut self.code, 1);
        self.code.push(OP_I64_XOR);
        Ok(())
    }

    fn emit_xor<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        self.code.push(OP_I64_XOR);
        Ok(())
    }

    fn emit_and<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        self.code.push(OP_I64_AND);
        Ok(())
    }

    fn emit_not<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        self.emit_formula(order, body)?;
        self.code.push(OP_I64_CONST); leb128_i64(&mut self.code, 0xFFFF_FFFF);
        self.code.push(OP_I64_XOR);
        Ok(())
    }

    fn emit_shl<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        self.code.push(OP_I64_SHL);
        self.code.push(OP_I64_CONST); leb128_i64(&mut self.code, 0xFFFF_FFFF);
        self.code.push(OP_I64_AND);
        Ok(())
    }

    // ── helpers ───────────────────────────────────────────────────

    /// if value (on stack, also in local_v) >= P: value - P, else value
    fn emit_reduce(&mut self, lv: u32) {
        // value already on stack — drop it, we'll use the local
        // Actually: value is on stack from local_tee. We need to check >= P.
        // But we consumed the stack value in the if condition above.
        // Let's just use the local.
        self.code.push(OP_LOCAL_GET); leb128_u32(&mut self.code, lv);
        self.code.push(OP_I64_CONST); leb128_i64(&mut self.code, P as i64);
        self.code.push(OP_I64_GE_U);
        self.code.push(OP_IF); self.code.push(TYPE_I64);
        self.code.push(OP_LOCAL_GET); leb128_u32(&mut self.code, lv);
        self.code.push(OP_I64_CONST); leb128_i64(&mut self.code, P as i64);
        self.code.push(OP_I64_SUB);
        self.code.push(OP_ELSE);
        self.code.push(OP_LOCAL_GET); leb128_u32(&mut self.code, lv);
        self.code.push(OP_END);
    }
}

// ── goldilocks_mul helper function ───────────────────────────────

/// Emit the body of goldilocks_mul(a: i64, b: i64) -> i64
/// Uses schoolbook 32×32→64 decomposition for the 128-bit product,
/// then reduces via 2^64 ≡ 2^32-1 (mod p).
fn emit_goldilocks_mul() -> (Vec<u8>, u32) {
    let mut c = Vec::with_capacity(200);
    // params: 0=a, 1=b
    // locals: 2=al, 3=ah, 4=bl, 5=bh, 6=lo, 7=hi, 8=mid, 9=tmp, 10=result
    let num_locals: u32 = 9;
    let mask32: i64 = 0xFFFF_FFFF;

    // al = a & mask
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 0);
    c.push(OP_I64_CONST); leb128_i64(&mut c, mask32);
    c.push(OP_I64_AND);
    c.push(OP_LOCAL_SET); leb128_u32(&mut c, 2); // al

    // ah = a >> 32
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 0);
    c.push(OP_I64_CONST); leb128_i64(&mut c, 32);
    c.push(OP_I64_SHR_U);
    c.push(OP_LOCAL_SET); leb128_u32(&mut c, 3); // ah

    // bl = b & mask
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 1);
    c.push(OP_I64_CONST); leb128_i64(&mut c, mask32);
    c.push(OP_I64_AND);
    c.push(OP_LOCAL_SET); leb128_u32(&mut c, 4); // bl

    // bh = b >> 32
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 1);
    c.push(OP_I64_CONST); leb128_i64(&mut c, 32);
    c.push(OP_I64_SHR_U);
    c.push(OP_LOCAL_SET); leb128_u32(&mut c, 5); // bh

    // lo = al * bl
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 2);
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 4);
    c.push(OP_I64_MUL);
    c.push(OP_LOCAL_SET); leb128_u32(&mut c, 6); // lo

    // hi = ah * bh
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 3);
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 5);
    c.push(OP_I64_MUL);
    c.push(OP_LOCAL_SET); leb128_u32(&mut c, 7); // hi

    // mid = ah * bl
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 3);
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 4);
    c.push(OP_I64_MUL);
    c.push(OP_LOCAL_SET); leb128_u32(&mut c, 8); // mid

    // hi += mid >> 32
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 7);
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 8);
    c.push(OP_I64_CONST); leb128_i64(&mut c, 32);
    c.push(OP_I64_SHR_U);
    c.push(OP_I64_ADD);
    c.push(OP_LOCAL_SET); leb128_u32(&mut c, 7);

    // tmp = lo + (mid & mask) << 32
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 6);
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 8);
    c.push(OP_I64_CONST); leb128_i64(&mut c, mask32);
    c.push(OP_I64_AND);
    c.push(OP_I64_CONST); leb128_i64(&mut c, 32);
    c.push(OP_I64_SHL);
    c.push(OP_I64_ADD);
    c.push(OP_LOCAL_TEE); leb128_u32(&mut c, 9); // tmp

    // carry: if tmp < lo then hi += 1
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 6);
    c.push(OP_I64_LT_U);
    c.push(OP_IF); c.push(0x40); // void block
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 7);
    c.push(OP_I64_CONST); leb128_i64(&mut c, 1);
    c.push(OP_I64_ADD);
    c.push(OP_LOCAL_SET); leb128_u32(&mut c, 7);
    c.push(OP_END);

    // lo = tmp
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 9);
    c.push(OP_LOCAL_SET); leb128_u32(&mut c, 6);

    // mid2 = al * bh
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 2);
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 5);
    c.push(OP_I64_MUL);
    c.push(OP_LOCAL_SET); leb128_u32(&mut c, 8); // reuse mid

    // hi += mid2 >> 32
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 7);
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 8);
    c.push(OP_I64_CONST); leb128_i64(&mut c, 32);
    c.push(OP_I64_SHR_U);
    c.push(OP_I64_ADD);
    c.push(OP_LOCAL_SET); leb128_u32(&mut c, 7);

    // tmp = lo + (mid2 & mask) << 32
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 6);
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 8);
    c.push(OP_I64_CONST); leb128_i64(&mut c, mask32);
    c.push(OP_I64_AND);
    c.push(OP_I64_CONST); leb128_i64(&mut c, 32);
    c.push(OP_I64_SHL);
    c.push(OP_I64_ADD);
    c.push(OP_LOCAL_TEE); leb128_u32(&mut c, 9);

    // carry: if tmp < lo then hi += 1
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 6);
    c.push(OP_I64_LT_U);
    c.push(OP_IF); c.push(0x40);
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 7);
    c.push(OP_I64_CONST); leb128_i64(&mut c, 1);
    c.push(OP_I64_ADD);
    c.push(OP_LOCAL_SET); leb128_u32(&mut c, 7);
    c.push(OP_END);

    // lo = tmp
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 9);
    c.push(OP_LOCAL_SET); leb128_u32(&mut c, 6);

    // Now reduce: result = lo + hi * (2^32 - 1) mod p
    // hi_shifted = hi * 0xFFFFFFFF  (= hi * 2^32 - hi)
    // But hi * 0xFFFFFFFF can overflow u64. Use: hi*(2^32) - hi = (hi<<32) - hi
    // sum = lo + (hi << 32) - hi
    // If this overflows, add 0xFFFFFFFF again.
    // Then reduce mod p.

    // result = lo
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 6);
    c.push(OP_LOCAL_SET); leb128_u32(&mut c, 10); // result

    // tmp = hi << 32
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 7);
    c.push(OP_I64_CONST); leb128_i64(&mut c, 32);
    c.push(OP_I64_SHL);
    c.push(OP_LOCAL_SET); leb128_u32(&mut c, 9);

    // result = result + tmp (may overflow)
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 10);
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 9);
    c.push(OP_I64_ADD);
    c.push(OP_LOCAL_TEE); leb128_u32(&mut c, 10);
    // carry if result < lo
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 6);
    c.push(OP_I64_LT_U);
    c.push(OP_IF); c.push(0x40);
    // overflow: result += 0xFFFFFFFF
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 10);
    c.push(OP_I64_CONST); leb128_i64(&mut c, mask32);
    c.push(OP_I64_ADD);
    c.push(OP_LOCAL_SET); leb128_u32(&mut c, 10);
    c.push(OP_END);

    // result = result - hi (may underflow)
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 10);
    c.push(OP_LOCAL_TEE); leb128_u32(&mut c, 9); // save before sub
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 7);
    c.push(OP_I64_SUB);
    c.push(OP_LOCAL_TEE); leb128_u32(&mut c, 10);
    // borrow if result > old_result
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 9);
    c.push(OP_I64_GT_U);
    c.push(OP_IF); c.push(0x40);
    // underflow: result -= 0xFFFFFFFF  (equiv: result += p - (p-1) adjustment)
    // Actually on underflow: result wrapped around. We need result + p.
    // But result + p might also overflow. For Goldilocks: just subtract 0xFFFFFFFF.
    // 2^64 - x + p = 2^64 - x + 2^64 - 2^32 + 1... hmm.
    // On underflow: result already wrapped (result = 2^64 + correct - old).
    // correct = old - hi, but we got old - hi + 2^64.
    // Need to subtract 2^64 mod p = subtract (2^32 - 1) = subtract 0xFFFFFFFF
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 10);
    c.push(OP_I64_CONST); leb128_i64(&mut c, mask32);
    c.push(OP_I64_SUB);
    c.push(OP_LOCAL_SET); leb128_u32(&mut c, 10);
    c.push(OP_END);

    // Final: if result >= P, subtract P
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 10);
    c.push(OP_I64_CONST); leb128_i64(&mut c, P as i64);
    c.push(OP_I64_GE_U);
    c.push(OP_IF); c.push(TYPE_I64);
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 10);
    c.push(OP_I64_CONST); leb128_i64(&mut c, P as i64);
    c.push(OP_I64_SUB);
    c.push(OP_ELSE);
    c.push(OP_LOCAL_GET); leb128_u32(&mut c, 10);
    c.push(OP_END);

    (c, num_locals)
}

const OP_I64_GT_U: u8 = 0x55;

// ── WASM module assembly ─────────────────────────────────────────

fn build_module(num_params: u32, formula: FormulaEmitter) -> Vec<u8> {
    let mut out = Vec::with_capacity(512);

    // Magic + version
    out.extend_from_slice(b"\0asm");
    out.extend_from_slice(&1u32.to_le_bytes());

    // Type section: 2 types
    // type 0: (i64, i64) → i64  (goldilocks_mul)
    // type 1: (i64 × num_params) → i64  (main)
    {
        let mut body = Vec::new();
        leb128_u32(&mut body, 2); // 2 types

        // type 0: mul helper
        body.push(TYPE_FUNC);
        leb128_u32(&mut body, 2);
        body.push(TYPE_I64); body.push(TYPE_I64);
        leb128_u32(&mut body, 1);
        body.push(TYPE_I64);

        // type 1: main
        body.push(TYPE_FUNC);
        leb128_u32(&mut body, num_params);
        for _ in 0..num_params { body.push(TYPE_I64); }
        leb128_u32(&mut body, 1);
        body.push(TYPE_I64);

        out.push(SEC_TYPE);
        leb128_u32(&mut out, body.len() as u32);
        out.extend_from_slice(&body);
    }

    // Function section: func 0 = type 0, func 1 = type 1
    {
        let mut body = Vec::new();
        leb128_u32(&mut body, 2);
        leb128_u32(&mut body, 0); // func 0 → type 0
        leb128_u32(&mut body, 1); // func 1 → type 1

        out.push(SEC_FUNCTION);
        leb128_u32(&mut out, body.len() as u32);
        out.extend_from_slice(&body);
    }

    // Export section: "main" → func 1
    {
        let mut body = Vec::new();
        leb128_u32(&mut body, 1);
        leb128_u32(&mut body, 4);
        body.extend_from_slice(b"main");
        body.push(0x00);
        leb128_u32(&mut body, 1); // func index 1

        out.push(SEC_EXPORT);
        leb128_u32(&mut out, body.len() as u32);
        out.extend_from_slice(&body);
    }

    // Code section: 2 function bodies
    {
        let (mul_code, mul_locals) = emit_goldilocks_mul();

        // Function 0: goldilocks_mul
        let mut func0 = Vec::new();
        if mul_locals > 0 {
            leb128_u32(&mut func0, 1);
            leb128_u32(&mut func0, mul_locals);
            func0.push(TYPE_I64);
        } else {
            leb128_u32(&mut func0, 0);
        }
        func0.extend_from_slice(&mul_code);
        func0.push(OP_END);

        // Function 1: main (formula)
        let mut func1 = Vec::new();
        if formula.num_locals > 0 {
            leb128_u32(&mut func1, 1);
            leb128_u32(&mut func1, formula.num_locals);
            func1.push(TYPE_I64);
        } else {
            leb128_u32(&mut func1, 0);
        }
        func1.extend_from_slice(&formula.code);
        func1.push(OP_END);

        let mut section = Vec::new();
        leb128_u32(&mut section, 2); // 2 bodies
        leb128_u32(&mut section, func0.len() as u32);
        section.extend_from_slice(&func0);
        leb128_u32(&mut section, func1.len() as u32);
        section.extend_from_slice(&func1);

        out.push(SEC_CODE);
        leb128_u32(&mut out, section.len() as u32);
        out.extend_from_slice(&section);
    }

    out
}

// ── LEB128 ───────────────────────────────────────────────────────

fn leb128_u32(buf: &mut Vec<u8>, mut val: u32) {
    loop {
        let byte = (val & 0x7F) as u8;
        val >>= 7;
        if val == 0 { buf.push(byte); break; }
        buf.push(byte | 0x80);
    }
}

fn leb128_i64(buf: &mut Vec<u8>, mut val: i64) {
    loop {
        let byte = (val & 0x7F) as u8;
        val >>= 7;
        if (val == 0 && byte & 0x40 == 0) || (val == -1 && byte & 0x40 != 0) {
            buf.push(byte); break;
        }
        buf.push(byte | 0x80);
    }
}
