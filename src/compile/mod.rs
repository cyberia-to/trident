//! nox native compilation — formula tree → native code
//!
//! Phase 1: atom-only formulas (no cells in output).
//! Subject = cons-list of atoms → function parameters.
//! Patterns 0,1,4,5-14 supported. 2,3,15-17 → Phase 2.

use nox::noun::{Order, NounId, Noun};

pub mod wasm;
pub mod arm64;
pub mod rv32;
pub mod rv64;
pub mod rvv;
pub mod x86_64;
pub mod ebpf;
pub mod ptx;
pub mod tensor_cores;
pub mod wgsl;
pub mod spirv;
pub mod amx;
pub mod ane;
pub mod thumb2;
pub mod hexagon;
pub mod verilog;
pub mod systemverilog;
pub mod vhdl;
pub mod qasm;
pub mod xla;
pub mod mir2nox;
pub mod qir;
pub mod onnx;
pub mod cerebras;
pub mod upmem;
pub mod intel_amx;

/// Compiled value — either a constant or derived from computation.
/// The compiler walks the formula tree and produces a sequence of
/// typed operations in the target's native format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompileError {
    /// Pattern not supported in Phase 1 (compose, cons, hash, call, look)
    UnsupportedPattern(u64),
    /// Malformed formula structure
    Malformed,
    /// Axis navigates into non-existent subject position
    AxisError(u64),
    /// Subject has no parameters (axis beyond available params)
    NoParams,
}

/// Resolve nox axis address to parameter index.
///
/// Subject layout: `[argN [argN-1 [... [arg1 0]]]]`
/// - axis 1 = whole subject (not a single param)
/// - axis 2 = head = param 0
/// - axis 6 = head of tail = param 1  (6 = 2*3)
/// - axis 14 = head of tail of tail = param 2 (14 = 2*7)
/// - axis 30 = param 3 (30 = 2*15)
///
/// Pattern: start at axis, walk bits from MSB.
/// Bit 0 = go left (head), bit 1 = go right (tail).
/// The last step must be head (bit 0) to reach an atom param.
pub fn axis_to_param(axis: u64) -> Result<u32, CompileError> {
    if axis < 2 {
        return Err(CompileError::AxisError(axis));
    }
    // Walk the binary representation of axis (skip leading 1)
    let bits = 64 - axis.leading_zeros() - 1; // number of navigation steps
    let mut depth = 0u32;
    for i in (0..bits).rev() {
        let bit = (axis >> i) & 1;
        if bit == 0 {
            // head — this should be the final step for a param access
            if i == 0 {
                return Ok(depth);
            }
            // head then more navigation — nested cell in subject
            // For Phase 1, subject is flat cons-list, so head is always atom
            return Err(CompileError::AxisError(axis));
        } else {
            // tail — go deeper into cons-list
            depth += 1;
        }
    }
    // Reached end without taking head — axis points to tail (the rest of list)
    Err(CompileError::AxisError(axis))
}

/// Extract pattern tag and body from a formula noun.
pub fn formula_parts<const N: usize>(order: &Order<N>, formula: NounId) -> Result<(u64, NounId), CompileError> {
    match order.get(formula).inner {
        Noun::Cell { left, right } => {
            match order.atom_value(left) {
                Some((v, _)) => Ok((v.as_u64(), right)),
                None => Err(CompileError::Malformed),
            }
        }
        Noun::Atom { .. } => Err(CompileError::Malformed),
    }
}

/// Extract binary op pair [a b] from body.
pub fn body_pair<const N: usize>(order: &Order<N>, body: NounId) -> Result<(NounId, NounId), CompileError> {
    match order.get(body).inner {
        Noun::Cell { left, right } => Ok((left, right)),
        _ => Err(CompileError::Malformed),
    }
}

/// Extract branch triple [test yes no] from body.
pub fn body_triple<const N: usize>(order: &Order<N>, body: NounId) -> Result<(NounId, NounId, NounId), CompileError> {
    let (test, rest) = body_pair(order, body)?;
    let (yes, no) = body_pair(order, rest)?;
    Ok((test, yes, no))
}

/// Get atom value from a noun (for quote literals).
pub fn atom_u64<const N: usize>(order: &Order<N>, r: NounId) -> Result<u64, CompileError> {
    match order.get(r).inner {
        Noun::Atom { value, .. } => Ok(value.as_u64()),
        _ => Err(CompileError::Malformed),
    }
}

/// Detect if a formula is a quote of an atom (pattern 1 with atom body).
pub fn is_quote_atom<const N: usize>(order: &Order<N>, formula: NounId) -> Option<u64> {
    let (tag, body) = formula_parts(order, formula).ok()?;
    if tag != 1 { return None; }
    atom_u64(order, body).ok()
}

/// Loop setup compose: `[2 [init_cons [1 loop_body]]]`
/// where init_cons = `[3 [[1 formula] [3 [init0 [3 [init1 ... [0 1]]]]]]]`
/// Returns (loop_body, list of init formulas for carried locals) if this is a loop.
pub fn detect_loop_setup<const N: usize>(
    order: &Order<N>, compose_body: NounId,
) -> Option<(NounId, Vec<NounId>)> {
    let (a, b) = body_pair(order, compose_body).ok()?;
    // B must be [1 body] (quoted)
    let (b_tag, loop_body) = formula_parts(order, b).ok()?;
    if b_tag != 1 { return None; }
    // A must be [3 [[1 formula] cons_tail]]
    let (a_tag, a_body) = formula_parts(order, a).ok()?;
    if a_tag != 3 { return None; }
    let (formula_slot, rest) = body_pair(order, a_body).ok()?;
    // formula_slot must be [1 X] (quoted formula)
    let (fs_tag, _) = formula_parts(order, formula_slot).ok()?;
    if fs_tag != 1 { return None; }
    // rest is a cons chain of init values ending with [0 1]
    let mut inits = Vec::new();
    let mut cur = rest;
    loop {
        let (tag, body) = formula_parts(order, cur).ok()?;
        if tag == 0 { break; } // [0 1] = identity, end of init chain
        if tag != 3 { return None; }
        let (val, tail) = body_pair(order, body).ok()?;
        inits.push(val);
        cur = tail;
    }
    Some((loop_body, inits))
}

/// Back-edge compose: `[2 [new_subj [0 N]]]` — code part is axis (not quoted).
/// Returns (new_subject_formula, axis_number) if this is a back-edge.
pub fn detect_back_edge<const N: usize>(
    order: &Order<N>, compose_body: NounId,
) -> Option<(NounId, u64)> {
    let (a, b) = body_pair(order, compose_body).ok()?;
    let (b_tag, b_body) = formula_parts(order, b).ok()?;
    if b_tag != 0 { return None; } // must be axis
    let axis = atom_u64(order, b_body).ok()?;
    Some((a, axis))
}

/// Extract carried-local init values and count from a loop's init cons chain.
/// Walks the cons chain built by mir2nox: cons(init_last, cons(init..., cons(init_first, params)))
pub fn count_loop_slots(inits: &[NounId]) -> u32 {
    inits.len() as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use nox::noun::{Order, Tag};
    use nebu::Goldilocks;

    fn g(v: u64) -> Goldilocks { Goldilocks::new(v) }

    /// Helper: build a formula noun in an order from a text-like structure.
    fn make_cell<const N: usize>(order: &mut Order<N>, left: NounId, right: NounId) -> NounId {
        order.cell(left, right).unwrap()
    }
    fn make_atom<const N: usize>(order: &mut Order<N>, v: u64) -> NounId {
        order.atom(g(v), Tag::Field).unwrap()
    }
    /// Build [tag body]
    fn make_formula<const N: usize>(order: &mut Order<N>, tag: u64, body: NounId) -> NounId {
        let t = make_atom(order, tag);
        make_cell(order, t, body)
    }
    /// Build [tag [left right]]
    fn make_binary<const N: usize>(order: &mut Order<N>, tag: u64, left: NounId, right: NounId) -> NounId {
        let body = make_cell(order, left, right);
        make_formula(order, tag, body)
    }

    #[test]
    fn axis_param_mapping() {
        assert_eq!(axis_to_param(2), Ok(0));
        assert_eq!(axis_to_param(6), Ok(1));
        assert_eq!(axis_to_param(14), Ok(2));
        assert_eq!(axis_to_param(30), Ok(3));
        assert_eq!(axis_to_param(62), Ok(4));
    }

    #[test]
    fn axis_errors() {
        assert!(axis_to_param(0).is_err());
        assert!(axis_to_param(1).is_err());
        assert!(axis_to_param(3).is_err());
    }

    #[test]
    fn wasm_quote_compiles() {
        let mut order = Order::<1024>::new();
        let body = make_atom(&mut order, 42);
        let formula = make_formula(&mut order, 1, body);
        let wasm = wasm::compile_to_wasm(&order, formula, 0).unwrap();
        // Valid WASM magic
        assert_eq!(&wasm[0..4], b"\0asm");
        assert!(wasm.len() > 8);
    }

    #[test]
    fn wasm_add_compiles() {
        let mut o = Order::<1024>::new();
        let a3 = make_atom(&mut o, 3);
        let q3 = make_formula(&mut o, 1, a3);
        let a5 = make_atom(&mut o, 5);
        let q5 = make_formula(&mut o, 1, a5);
        let formula = make_binary(&mut o, 5, q3, q5);
        let wasm = wasm::compile_to_wasm(&o, formula, 0).unwrap();
        assert_eq!(&wasm[0..4], b"\0asm");
    }

    #[test]
    fn wasm_axis_param_compiles() {
        let mut o = Order::<1024>::new();
        let ax2 = make_atom(&mut o, 2);
        let a0 = make_formula(&mut o, 0, ax2);
        let ax6 = make_atom(&mut o, 6);
        let a1 = make_formula(&mut o, 0, ax6);
        let formula = make_binary(&mut o, 5, a0, a1);
        let wasm = wasm::compile_to_wasm(&o, formula, 2).unwrap();
        assert_eq!(&wasm[0..4], b"\0asm");
    }

    #[test]
    fn wasm_branch_compiles() {
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
        let wasm = wasm::compile_to_wasm(&o, formula, 0).unwrap();
        assert_eq!(&wasm[0..4], b"\0asm");
    }

    #[test]
    fn wasm_mul_compiles() {
        let mut o = Order::<1024>::new();
        let ax2 = make_atom(&mut o, 2);
        let a0 = make_formula(&mut o, 0, ax2);
        let ax6 = make_atom(&mut o, 6);
        let a1 = make_formula(&mut o, 0, ax6);
        let formula = make_binary(&mut o, 7, a0, a1);
        let wasm = wasm::compile_to_wasm(&o, formula, 2).unwrap();
        assert_eq!(&wasm[0..4], b"\0asm");
    }

    #[test]
    fn arm64_add_compiles() {
        let mut o = Order::<1024>::new();
        let ax2 = make_atom(&mut o, 2);
        let a0 = make_formula(&mut o, 0, ax2);
        let ax6 = make_atom(&mut o, 6);
        let a1 = make_formula(&mut o, 0, ax6);
        let formula = make_binary(&mut o, 5, a0, a1);
        let code = arm64::compile_to_arm64(&o, formula, 2).unwrap();
        let len = code.len();
        assert!(len >= 4);
        let last4 = u32::from_le_bytes([code[len-4], code[len-3], code[len-2], code[len-1]]);
        assert_eq!(last4, 0xD65F03C0, "should end with RET");
    }

    #[test]
    fn arm64_mul_compiles() {
        let mut o = Order::<1024>::new();
        let ax2 = make_atom(&mut o, 2);
        let a0 = make_formula(&mut o, 0, ax2);
        let ax6 = make_atom(&mut o, 6);
        let a1 = make_formula(&mut o, 0, ax6);
        let formula = make_binary(&mut o, 7, a0, a1);
        let code = arm64::compile_to_arm64(&o, formula, 2).unwrap();
        let len = code.len();
        let last4 = u32::from_le_bytes([code[len-4], code[len-3], code[len-2], code[len-1]]);
        assert_eq!(last4, 0xD65F03C0, "should end with RET");
    }

    #[test]
    fn x86_64_add_compiles() {
        let mut o = Order::<1024>::new();
        let ax2 = make_atom(&mut o, 2);
        let a0 = make_formula(&mut o, 0, ax2);
        let ax6 = make_atom(&mut o, 6);
        let a1 = make_formula(&mut o, 0, ax6);
        let formula = make_binary(&mut o, 5, a0, a1);
        let code = x86_64::compile_to_x86_64(&o, formula, 2).unwrap();
        // Should end with RET (0xC3)
        assert!(!code.is_empty());
        assert_eq!(*code.last().unwrap(), 0xC3, "should end with RET");
    }

    #[test]
    fn x86_64_mul_compiles() {
        let mut o = Order::<1024>::new();
        let ax2 = make_atom(&mut o, 2);
        let a0 = make_formula(&mut o, 0, ax2);
        let ax6 = make_atom(&mut o, 6);
        let a1 = make_formula(&mut o, 0, ax6);
        let formula = make_binary(&mut o, 7, a0, a1);
        let code = x86_64::compile_to_x86_64(&o, formula, 2).unwrap();
        assert_eq!(*code.last().unwrap(), 0xC3, "should end with RET");
    }

    #[test]
    fn rv32_add_compiles() {
        let mut o = Order::<1024>::new();
        let ax2 = make_atom(&mut o, 2);
        let a0 = make_formula(&mut o, 0, ax2);
        let ax6 = make_atom(&mut o, 6);
        let a1 = make_formula(&mut o, 0, ax6);
        let formula = make_binary(&mut o, 5, a0, a1);
        let code = rv32::compile_to_rv32(&o, formula, 2).unwrap();
        let len = code.len();
        assert!(len >= 4);
        // Should end with RET = JALR x0, x1, 0 = 0x00008067
        let last4 = u32::from_le_bytes([code[len-4], code[len-3], code[len-2], code[len-1]]);
        assert_eq!(last4, 0x00008067, "should end with RET");
    }

    #[test]
    fn rv64_add_compiles() {
        let mut o = Order::<1024>::new();
        let ax2 = make_atom(&mut o, 2);
        let a0 = make_formula(&mut o, 0, ax2);
        let ax6 = make_atom(&mut o, 6);
        let a1 = make_formula(&mut o, 0, ax6);
        let formula = make_binary(&mut o, 5, a0, a1);
        let code = rv64::compile_to_rv64(&o, formula, 2).unwrap();
        let len = code.len();
        assert!(len >= 4);
        // Should end with RET = JALR x0, x1, 0 = 0x00008067
        let last4 = u32::from_le_bytes([code[len-4], code[len-3], code[len-2], code[len-1]]);
        assert_eq!(last4, 0x00008067, "should end with RET");
    }

    #[test]
    fn ebpf_add_compiles() {
        let mut o = Order::<1024>::new();
        let ax2 = make_atom(&mut o, 2);
        let a0 = make_formula(&mut o, 0, ax2);
        let ax6 = make_atom(&mut o, 6);
        let a1 = make_formula(&mut o, 0, ax6);
        let formula = make_binary(&mut o, 5, a0, a1);
        let code = ebpf::compile_to_ebpf(&o, formula, 2).unwrap();
        // Should end with EXIT = 0x95
        let len = code.len();
        assert!(len >= 8);
        let last8 = u64::from_le_bytes(code[len-8..len].try_into().unwrap());
        assert_eq!(last8 & 0xFF, 0x95, "should end with EXIT");
    }

    #[test]
    fn unsupported_pattern_errors() {
        let mut o = Order::<1024>::new();
        let a1 = make_atom(&mut o, 1);
        let body = make_formula(&mut o, 0, a1);
        let formula = make_formula(&mut o, 15, body);
        assert!(matches!(
            wasm::compile_to_wasm(&o, formula, 1),
            Err(CompileError::UnsupportedPattern(15))
        ));
    }
}
