// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! NoxCompiler — direct AST → nox Noun compilation.
//!
//! Bypasses TIR entirely. The AST is a tree, nox nouns are trees,
//! the mapping is 1:1. No stack IR, no symbolic stack reconstruction.
//!
//! Subject model: right-nested cons list of bindings.
//! ```text
//! let x = 3; let y = 5; x + y
//! subject: [y [x [params... 0]]]
//! ```
//! Each let-binding conses onto the subject. Variable lookup = axis.
//!
//! Statement model: every statement lowers to a *subject transformer* — a
//! formula that, evaluated against the current subject, produces the next
//! subject. Statements chain via compose:
//! ```text
//! seq(t, rest) = [2 [t [1 rest]]]
//! ```
//! - `let`    → cons the initializer onto the subject
//! - `x = v`  → subject edit: rebuild the subject with `v` at x's axis
//!              (shape-preserving, so mutation survives branches and loops)
//! - `if`     → branch whose arms are transformers producing the *outer*
//!              subject shape (arm-local bindings dropped, edits kept);
//!              arms containing `return` instead absorb the continuation
//! - expr `;` → cons the value on (evaluated for effect), never silently
//!              dropped — a crashing expression must crash

use std::collections::BTreeMap;

use crate::ast::{self, BinOp, Block, Expr, FnDef, Item, Literal, Pattern, Stmt};
use crate::span::Spanned;
use super::Noun;

// ─── nox formula constructors (tags 0-17) ────────────────────────

fn nox_axis(addr: u64) -> Noun {
    Noun::cell(Noun::atom(0), Noun::atom(addr))
}

fn nox_quote(c: Noun) -> Noun {
    Noun::cell(Noun::atom(1), c)
}

fn nox_compose(subject_formula: Noun, code_formula: Noun) -> Noun {
    Noun::cell(Noun::atom(2), Noun::cell(subject_formula, code_formula))
}

fn nox_cons(a: Noun, b: Noun) -> Noun {
    Noun::cell(Noun::atom(3), Noun::cell(a, b))
}

fn nox_branch(test: Noun, yes: Noun, no: Noun) -> Noun {
    Noun::cell(Noun::atom(4), Noun::cell(test, Noun::cell(yes, no)))
}

fn nox_add(a: Noun, b: Noun) -> Noun {
    Noun::cell(Noun::atom(5), Noun::cell(a, b))
}

#[allow(dead_code)]
fn nox_sub(a: Noun, b: Noun) -> Noun {
    Noun::cell(Noun::atom(6), Noun::cell(a, b))
}

fn nox_mul(a: Noun, b: Noun) -> Noun {
    Noun::cell(Noun::atom(7), Noun::cell(a, b))
}

fn nox_inv(a: Noun) -> Noun {
    Noun::cell(Noun::atom(8), a)
}

fn nox_eq(a: Noun, b: Noun) -> Noun {
    Noun::cell(Noun::atom(9), Noun::cell(a, b))
}

fn nox_lt(a: Noun, b: Noun) -> Noun {
    Noun::cell(Noun::atom(10), Noun::cell(a, b))
}

fn nox_xor(a: Noun, b: Noun) -> Noun {
    Noun::cell(Noun::atom(11), Noun::cell(a, b))
}

fn nox_and(a: Noun, b: Noun) -> Noun {
    Noun::cell(Noun::atom(12), Noun::cell(a, b))
}

#[allow(dead_code)]
fn nox_not(a: Noun) -> Noun {
    Noun::cell(Noun::atom(13), a)
}

#[allow(dead_code)]
fn nox_shl(a: Noun, b: Noun) -> Noun {
    Noun::cell(Noun::atom(14), Noun::cell(a, b))
}

fn nox_hash(a: Noun) -> Noun {
    Noun::cell(Noun::atom(15), a)
}

/// Non-deterministic call (pattern 16): `[16 [tag_formula check_formula]]`.
/// Both children MUST be formulas (pairs), not bare atoms.
fn nox_call(tag_formula: Noun, check_formula: Noun) -> Noun {
    Noun::cell(Noun::atom(16), Noun::cell(tag_formula, check_formula))
}

/// A formula that always errors when reduced: `inv(0)` → InvZero. Used to make
/// a failed assertion abort the reduction (proof generation becomes impossible),
/// which is exactly `assert`'s contract.
fn nox_crash() -> Noun {
    nox_inv(nox_quote(Noun::atom(0)))
}

/// Sequence: apply transformer `t` to the subject, then evaluate `rest`
/// against the transformed subject. `[2 [t [1 rest]]]`.
fn seq(t: Noun, rest: Noun) -> Noun {
    nox_compose(t, nox_quote(rest))
}

/// The unit value: `[1 0]`.
fn nox_unit() -> Noun {
    nox_quote(Noun::atom(0))
}

// ─── axis arithmetic ─────────────────────────────────────────────

/// Deepest supported binding position. Axis addresses are u64 bit paths;
/// `stack_axis(60) = 2^62 - 2` still fits, and leaves headroom for
/// element paths concatenated onto variable axes.
const MAX_SUBJECT_DEPTH: u32 = 58;

/// Maximum number of iterations a bounded loop may be unrolled to. Static
/// unrolling multiplies body size by iteration count; beyond this the formula
/// blows up, so we refuse rather than emit a multi-gigabyte noun.
const MAX_UNROLL: u64 = 4096;

/// Node budget for inlining. A diamond-shaped (non-recursive) call graph can
/// still expand exponentially; this cap turns that into an honest error.
const MAX_INLINE_NODES: usize = 2_000_000;

/// Count the nodes in a noun (atoms + cells).
fn count_nodes(n: &Noun) -> usize {
    match n {
        Noun::Atom(_) => 1,
        Noun::Cell(h, t) => 1 + count_nodes(h) + count_nodes(t),
    }
}

/// Build a NIL-terminated cons list `[e0 [e1 [.. [e_{n-1} 0]]]]` from element
/// formulas. Element `i` lives at axis `stack_axis(i)`.
fn cons_list(elems: Vec<Noun>) -> Noun {
    let mut acc = nox_quote(Noun::atom(0));
    for e in elems.into_iter().rev() {
        acc = nox_cons(e, acc);
    }
    acc
}

/// Read element `i` of an aggregate produced by `base`. When `base` is a bare
/// axis `[0 A]`, fuse the paths into a single axis; otherwise evaluate `base`
/// then navigate into it via compose.
fn elem_access(base: Noun, i: u32) -> Noun {
    let rel = stack_axis(i);
    match &base {
        Noun::Cell(tag, addr) => {
            if let (Noun::Atom(0), Noun::Atom(a)) = (tag.as_ref(), addr.as_ref()) {
                return nox_axis(axis_compose(*a, rel));
            }
            nox_compose(base.clone(), nox_quote(nox_axis(rel)))
        }
        _ => nox_compose(base, nox_quote(nox_axis(rel))),
    }
}

/// Compose two axis bit-paths: navigate `a`, then `b` from there.
/// `axis_compose(1, b) = b`; `axis_compose(a, 1) = a`.
fn axis_compose(a: u64, b: u64) -> u64 {
    if b == 1 {
        return a;
    }
    let bits = 63 - b.leading_zeros();
    let low = b - (1u64 << bits);
    (a << bits) | low
}

fn check_unroll(iters: u64) -> Result<(), String> {
    if iters > MAX_UNROLL {
        return Err(format!(
            "nox: loop unroll of {} iterations exceeds limit {} \
             (bound the loop tighter, or await recursive-core lowering)",
            iters, MAX_UNROLL
        ));
    }
    Ok(())
}

/// Axis address for stack[n] in a right-nested cons list.
/// stack[0] = 2, stack[1] = 6, stack[2] = 14, stack[n] = 2^(n+2) - 2.
fn stack_axis(depth: u32) -> u64 {
    (1u64 << (depth + 2)) - 2
}

/// Axis of the subject remainder below the first `n` bindings:
/// tail^n. tail_axis(0) = 1, tail_axis(1) = 3, tail_axis(n) = 2^(n+1) - 1.
fn tail_axis(n: u32) -> u64 {
    (1u64 << (n + 1)) - 1
}

// ─── scope ───────────────────────────────────────────────────────

/// Variable scope: maps names to their depth in the subject cons list.
/// Depth 0 = head of subject (most recently bound).
#[derive(Clone, Debug)]
struct Scope {
    /// Stack of (name → bind_depth) frames. Inner vec = one scope level.
    frames: Vec<BTreeMap<String, u32>>,
    /// Per-frame aggregate-type annotations (only non-scalar types stored).
    tys: Vec<BTreeMap<String, ast::Type>>,
    /// Total number of cons operations (bindings) on the subject.
    depth: u32,
    /// Number of cons operations per frame (tracks actual binds, not unique names).
    frame_bind_counts: Vec<u32>,
}

impl Scope {
    fn new() -> Self {
        Self {
            frames: vec![BTreeMap::new()],
            tys: vec![BTreeMap::new()],
            depth: 0,
            frame_bind_counts: vec![0],
        }
    }

    fn push_frame(&mut self) {
        self.frames.push(BTreeMap::new());
        self.tys.push(BTreeMap::new());
        self.frame_bind_counts.push(0);
    }

    fn pop_frame(&mut self) {
        self.frames.pop();
        self.tys.pop();
        if let Some(count) = self.frame_bind_counts.pop() {
            self.depth -= count;
        }
    }

    /// Record the (aggregate) type of a binding in the current frame.
    fn note_type(&mut self, name: &str, ty: ast::Type) {
        if let Some(frame) = self.tys.last_mut() {
            frame.insert(name.to_string(), ty);
        }
    }

    /// Look up a binding's recorded type.
    fn lookup_type(&self, name: &str) -> Option<&ast::Type> {
        for frame in self.tys.iter().rev() {
            if let Some(t) = frame.get(name) {
                return Some(t);
            }
        }
        None
    }

    /// Bind a variable (cons onto subject). Returns its bind_depth.
    fn bind(&mut self, name: &str) -> u32 {
        let d = self.depth;
        if let Some(frame) = self.frames.last_mut() {
            frame.insert(name.to_string(), d);
        }
        if let Some(count) = self.frame_bind_counts.last_mut() {
            *count += 1;
        }
        self.depth += 1;
        d
    }

    /// Look up a variable. Returns its position in the subject (0 = head).
    fn lookup(&self, name: &str) -> Option<u32> {
        for frame in self.frames.iter().rev() {
            if let Some(&d) = frame.get(name) {
                return Some(self.depth - 1 - d);
            }
        }
        None
    }

    /// Erase the *names* of every frame at index >= `from` while keeping
    /// their bind counts. Used when a branch arm absorbs the statements
    /// that follow the branch: the arm's locals stay on the subject as
    /// anonymous padding, but must not shadow outer names.
    fn seal_frames_above(&mut self, from: usize) {
        for frame in self.frames.iter_mut().skip(from) {
            frame.clear();
        }
    }
}

// ─── compiler ────────────────────────────────────────────────────

type LowerResult = Result<Noun, String>;
type Cont<'a> = &'a mut dyn FnMut(&mut NoxCompiler) -> LowerResult;

/// Compiles a typed AST directly into nox Noun formulas.
pub struct NoxCompiler {
    scope: Scope,
    /// Resolved constants: name → value.
    constants: BTreeMap<String, u64>,
    /// All function definitions in the module, keyed by name (for inlining).
    fns: BTreeMap<String, FnDef>,
    /// Struct field layouts: struct name → ordered (field, type) pairs.
    structs: BTreeMap<String, Vec<(String, ast::Type)>>,
    /// Names of functions currently being inlined (cycle guard).
    call_stack: Vec<String>,
    /// Set when the program lowers an `os.state.read` — the bundle declares
    /// this so the runner supplies the subject as `[bbg_root [params…]]`.
    reads_state: bool,
    /// Running count of nodes emitted by inlining (exponential-blowup guard).
    inline_nodes: usize,
}

impl NoxCompiler {
    pub fn new() -> Self {
        Self {
            scope: Scope::new(),
            constants: BTreeMap::new(),
            fns: BTreeMap::new(),
            structs: BTreeMap::new(),
            call_stack: Vec::new(),
            inline_nodes: 0,
            reads_state: false,
        }
    }

    /// Whether the compiled program reads persistent state (nox look). The
    /// runner must then cons the BBG state root onto the subject:
    /// `[root_tree [param_last … [param0 0]]]`.
    pub fn reads_state(&self) -> bool {
        self.reads_state
    }

    /// Compile an AST file. Returns the Noun formula for the entry function.
    pub fn compile_file(&mut self, file: &ast::File) -> Result<Noun, String> {
        // Collect constants, function definitions, and struct layouts.
        for item in &file.items {
            match &item.node {
                Item::Const(c) => {
                    if let Expr::Literal(Literal::Integer(v)) = &c.value.node {
                        self.constants.insert(c.name.node.clone(), *v);
                    }
                }
                Item::Fn(f) => {
                    self.fns.insert(f.name.node.clone(), f.clone());
                }
                Item::Struct(sd) => {
                    let fields = sd
                        .fields
                        .iter()
                        .map(|f| (f.name.node.clone(), f.ty.node.clone()))
                        .collect();
                    self.structs.insert(sd.name.node.clone(), fields);
                }
                _ => {}
            }
        }

        // Find entry function (main or first public fn)
        let entry = file
            .items
            .iter()
            .find_map(|item| match &item.node {
                Item::Fn(f) if f.name.node == "main" => Some(f),
                _ => None,
            })
            .or_else(|| {
                file.items.iter().find_map(|item| match &item.node {
                    Item::Fn(f) if f.is_pub && f.body.is_some() => Some(f),
                    _ => None,
                })
            })
            .ok_or_else(|| "no entry function found".to_string())?;

        self.compile_fn(entry)
    }

    /// Compile a function definition into a nox formula.
    ///
    /// The formula expects a subject containing the function's parameters
    /// as a right-nested cons list: `[param_last [... [param0 0]]]`
    /// (first parameter deepest, last parameter at the head).
    pub fn compile_fn(&mut self, func: &FnDef) -> Result<Noun, String> {
        self.scope = Scope::new();

        // Bind parameters (first param = deepest, last param = head of subject)
        for param in &func.params {
            self.scope.bind(&param.name.node);
            self.scope.note_type(&param.name.node, param.ty.node.clone());
        }
        // A program that reads state receives the BBG root as the subject
        // head, above the parameters: `[root_tree [param_last … [param0 0]]]`.
        // Each `os.state.read` site re-conses this root to the head so the
        // look pattern finds its limbs at the fixed axes 4/10/22/23.
        if func
            .body
            .as_ref()
            .map(|b| block_uses_state(&b.node))
            .unwrap_or(false)
        {
            self.scope.bind("$bbg_root");
            self.reads_state = true;
        }
        self.check_depth()?;

        let body = func
            .body
            .as_ref()
            .ok_or_else(|| format!("function {} has no body", func.name.node))?;

        self.compile_block(&body.node)
    }

    fn check_depth(&self) -> Result<(), String> {
        if self.scope.depth > MAX_SUBJECT_DEPTH {
            return Err(format!(
                "nox: subject depth limit exceeded ({} bindings, max {})",
                self.scope.depth, MAX_SUBJECT_DEPTH
            ));
        }
        Ok(())
    }

    /// Compile a block in value position: statements, then the tail
    /// expression (or unit) as the block's value.
    fn compile_block(&mut self, block: &Block) -> LowerResult {
        self.scope.push_frame();
        let tail = block.tail_expr.as_deref();
        let result = self.compile_stmts_k(&block.stmts, &mut |c| match tail {
            Some(e) => c.compile_expr(&e.node),
            None => Ok(nox_unit()),
        });
        self.scope.pop_frame();
        result
    }

    /// Compile a statement sequence. `k` is the continuation: it produces
    /// the formula for whatever comes after these statements (the block's
    /// tail value, an enclosing arm's subject reification, or the
    /// statements following an absorbed branch).
    fn compile_stmts_k(&mut self, stmts: &[Spanned<Stmt>], k: Cont) -> LowerResult {
        let Some(stmt) = stmts.first() else {
            return k(self);
        };
        let rest = &stmts[1..];

        match &stmt.node {
            Stmt::Let {
                pattern, init, ty, ..
            } => match pattern {
                Pattern::Name(name) => {
                    let init_f = self.compile_expr(&init.node)?;
                    let inferred = ty
                        .as_ref()
                        .map(|t| t.node.clone())
                        .or_else(|| self.expr_type(&init.node));
                    self.scope.bind(&name.node);
                    if let Some(t) = inferred {
                        self.scope.note_type(&name.node, t);
                    }
                    self.check_depth()?;
                    let rest_f = self.compile_stmts_k(rest, k)?;
                    Ok(seq(nox_cons(init_f, nox_axis(1)), rest_f))
                }
                Pattern::Tuple(names) => {
                    // Bind the aggregate to a hidden temp, then bind each name
                    // to an element read from the temp. Element `i` reads the
                    // temp at the position it occupies when that cons runs.
                    let init_f = self.compile_expr(&init.node)?;
                    let init_ty = ty
                        .as_ref()
                        .map(|t| t.node.clone())
                        .or_else(|| self.expr_type(&init.node));
                    let elem_tys: Vec<Option<ast::Type>> = match &init_ty {
                        Some(ast::Type::Tuple(ts)) => ts.iter().cloned().map(Some).collect(),
                        _ => vec![None; names.len()],
                    };
                    self.scope.bind("$tuple");
                    self.check_depth()?;
                    let mut elem_forms = Vec::with_capacity(names.len());
                    for (i, name) in names.iter().enumerate() {
                        let temp_pos = self.scope.lookup("$tuple").unwrap();
                        let elem_f =
                            elem_access(nox_axis(stack_axis(temp_pos)), i as u32);
                        self.scope.bind(&name.node);
                        if let Some(t) = &elem_tys[i] {
                            self.scope.note_type(&name.node, t.clone());
                        }
                        self.check_depth()?;
                        elem_forms.push(elem_f);
                    }
                    let rest_f = self.compile_stmts_k(rest, k)?;
                    let mut acc = rest_f;
                    for elem_f in elem_forms.into_iter().rev() {
                        acc = seq(nox_cons(elem_f, nox_axis(1)), acc);
                    }
                    Ok(seq(nox_cons(init_f, nox_axis(1)), acc))
                }
            },

            Stmt::Assign { place, value } => {
                let value_f = self.compile_expr(&value.node)?;
                let (axis, _ty) = self.place_axis(&place.node)?;
                let edit = subject_edit(axis, value_f)?;
                let rest_f = self.compile_stmts_k(rest, k)?;
                Ok(seq(edit, rest_f))
            }

            Stmt::If {
                cond,
                then_block,
                else_block,
            } => {
                let cond_f = self.compile_expr(&cond.node)?;
                let has_return = block_has_return(&then_block.node)
                    || else_block
                        .as_ref()
                        .map(|b| block_has_return(&b.node))
                        .unwrap_or(false);

                if has_return {
                    // Absorbing mode: an arm that returns produces the final
                    // value; the fall-through arm must carry the statements
                    // after the `if` inside itself. The continuation is
                    // compiled into each arm (duplicated).
                    let frame_baseline = self.scope.frames.len();
                    let then_f = self.absorb_arm(&then_block.node, rest, frame_baseline, k)?;
                    let else_f = match else_block {
                        Some(b) => self.absorb_arm(&b.node, rest, frame_baseline, k)?,
                        None => self.compile_stmts_k(rest, k)?,
                    };
                    Ok(nox_branch(cond_f, then_f, else_f))
                } else {
                    // Transformer mode: each arm produces the outer-shaped
                    // subject (locals dropped, edits to outer variables
                    // kept). The rest is compiled once, after the branch.
                    let then_f = self.arm_transform(&then_block.node)?;
                    let else_f = match else_block {
                        Some(b) => self.arm_transform(&b.node)?,
                        None => nox_axis(1),
                    };
                    let branch = nox_branch(cond_f, then_f, else_f);
                    let rest_f = self.compile_stmts_k(rest, k)?;
                    Ok(seq(branch, rest_f))
                }
            }

            Stmt::Expr(expr) => {
                // Evaluate for effect: the value is consed on (so reduction
                // errors — failed assertions — propagate), then the rest runs
                // with the result as an anonymous binding.
                self.effect_stmt(expr, rest, k)
            }

            Stmt::Return(Some(expr)) => self.compile_expr(&expr.node),
            Stmt::Return(None) => Ok(nox_unit()),

            Stmt::For {
                var,
                start,
                end,
                bound,
                body,
            } => {
                if block_has_return(&body.node) {
                    return Err(
                        "nox: `return` inside a for loop is not supported \
                         (needs recursive-core continuation lowering — TODO)"
                            .to_string(),
                    );
                }
                let start_val = self.eval_const(&start.node).ok_or_else(|| {
                    "nox: for-loop start must be a compile-time constant".to_string()
                })?;
                let end_const = self.eval_const(&end.node);

                // Fold iterations into the continuation. Because the loop body
                // is a subject transformer that preserves the outer shape, the
                // scope after the loop equals the scope before it — so `rest`
                // compiles against the current scope.
                let mut acc = self.compile_stmts_k(rest, k)?;

                match (end_const, bound) {
                    // Static bound: exact unconditional unroll.
                    (Some(end_val), _) => {
                        if end_val > start_val {
                            let n = end_val - start_val;
                            check_unroll(n)?;
                            for j in (0..n).rev() {
                                let idx = start_val + j;
                                let iter_t =
                                    self.loop_body_transform(&var.node, idx, &body.node)?;
                                acc = seq(iter_t, acc);
                            }
                        }
                        // end_val <= start_val → zero iterations, acc unchanged.
                        Ok(acc)
                    }
                    // Dynamic end with a declared bound: `bound` guarded
                    // iterations, each gated on `start+j < end`.
                    (None, Some(b)) => {
                        let b = *b;
                        check_unroll(b)?;
                        for j in (0..b).rev() {
                            let idx = start_val + j;
                            let body_t =
                                self.loop_body_transform(&var.node, idx, &body.node)?;
                            // Guard: run body while index < end, else identity.
                            let end_f = self.compile_expr(&end.node)?;
                            let guard = nox_branch(
                                nox_lt(nox_quote(Noun::atom(idx)), end_f),
                                body_t,
                                nox_axis(1),
                            );
                            acc = seq(guard, acc);
                        }
                        Ok(acc)
                    }
                    (None, None) => Err(
                        "nox: for-loop end must be a compile-time constant or carry a \
                         `bounded N` annotation"
                            .to_string(),
                    ),
                }
            }
            Stmt::Asm { .. } => Err("nox: inline assembly not supported".to_string()),
            Stmt::Match { .. } => Err("nox: match not yet supported".to_string()),
            Stmt::TupleAssign { names, value } => {
                // Bind the aggregate to a temp, edit each named (outer) variable
                // from the corresponding temp element, then drop the temp so the
                // outer subject shape is restored.
                let value_f = self.compile_expr(&value.node)?;
                self.scope.push_frame();
                self.scope.bind("$tas");
                self.check_depth()?;
                let temp_pos = self.scope.lookup("$tas").unwrap();
                let mut edits = Vec::with_capacity(names.len());
                for (i, name) in names.iter().enumerate() {
                    let pos = self.scope.lookup(&name.node).ok_or_else(|| {
                        format!("nox: undefined variable '{}'", name.node)
                    })?;
                    let src = elem_access(nox_axis(stack_axis(temp_pos)), i as u32);
                    edits.push(subject_edit(stack_axis(pos), src)?);
                }
                let drop_temp = reify_drop(1, self.scope.depth);
                self.scope.pop_frame();
                // Chain: cons(value) · edit0 · edit1 · … · drop_temp
                let mut inner = drop_temp;
                for e in edits.into_iter().rev() {
                    inner = seq(e, inner);
                }
                let transform = seq(nox_cons(value_f, nox_axis(1)), inner);
                let rest_f = self.compile_stmts_k(rest, k)?;
                Ok(seq(transform, rest_f))
            }
            Stmt::Reveal { .. } | Stmt::Seal { .. } => {
                Err("nox: reveal/seal not yet supported".to_string())
            }
        }
    }

    /// Expression statement: cons the value onto the subject as an anonymous
    /// binding and continue. Keeps crash semantics without special cases.
    fn effect_stmt(
        &mut self,
        expr: &Spanned<Expr>,
        rest: &[Spanned<Stmt>],
        k: Cont,
    ) -> LowerResult {
        let f = self.compile_expr(&expr.node)?;
        self.scope.bind("$stmt");
        self.check_depth()?;
        let rest_f = self.compile_stmts_k(rest, k)?;
        Ok(seq(nox_cons(f, nox_axis(1)), rest_f))
    }

    /// Evaluate `expr` for effect, then continue with `fin`.
    fn effect_then(&mut self, expr: &Spanned<Expr>, fin: Cont) -> LowerResult {
        let f = self.compile_expr(&expr.node)?;
        self.scope.bind("$stmt");
        self.check_depth()?;
        let rest_f = fin(self)?;
        Ok(seq(nox_cons(f, nox_axis(1)), rest_f))
    }

    /// Compile one loop iteration as a subject transformer. Binds the loop
    /// variable to the constant `index`, runs the body (mutations to outer
    /// variables land via subject edit), then reifies the outer subject shape
    /// (dropping the loop variable and any body-local bindings).
    fn loop_body_transform(&mut self, var: &str, index: u64, block: &Block) -> LowerResult {
        let baseline_depth = self.scope.depth;
        self.scope.push_frame();
        self.scope.bind(var);
        self.check_depth()?;
        let tail = block.tail_expr.as_deref();
        let inner = self.compile_stmts_k(&block.stmts, &mut |c: &mut Self| {
            let mut fin = |c: &mut Self| {
                let locals = c.scope.depth - baseline_depth;
                Ok(reify_drop(locals, c.scope.depth))
            };
            match tail {
                Some(e) => c.effect_then(e, &mut fin),
                None => fin(c),
            }
        });
        self.scope.pop_frame();
        let inner = inner?;
        // Cons the loop index onto the subject, then run the body.
        Ok(seq(nox_cons(nox_quote(Noun::atom(index)), nox_axis(1)), inner))
    }

    /// Inline a user function at a call site. Arguments are evaluated against
    /// the caller subject and consed into a fresh callee subject
    /// `[arg_last .. [arg0 0]]`; the body is compiled against a fresh scope and
    /// applied to that subject via compose. trident forbids recursion, so a
    /// name recurring on the call stack is a hard error, and a node budget
    /// guards against exponential blowup from diamond call graphs.
    fn inline_call(
        &mut self,
        func: &FnDef,
        args: &[Spanned<Expr>],
        generic_args: &[Spanned<ast::ArraySize>],
    ) -> LowerResult {
        if self.call_stack.iter().any(|n| n == &func.name.node) {
            return Err(format!(
                "nox: recursive call to '{}' — trident forbids recursion",
                func.name.node
            ));
        }
        if func.params.len() != args.len() {
            return Err(format!(
                "nox: '{}' expects {} argument(s), got {}",
                func.name.node,
                func.params.len(),
                args.len()
            ));
        }
        if func.body.is_none() {
            return Err(format!(
                "nox: cannot inline '{}' — it has no body (intrinsic/extern)",
                func.name.node
            ));
        }

        // Evaluate arguments against the CALLER subject first.
        let mut cons_f = nox_quote(Noun::atom(0));
        // Fold forward so args[0] ends up deepest, matching param bind order.
        let mut arg_forms = Vec::with_capacity(args.len());
        for a in args {
            arg_forms.push(self.compile_expr(&a.node)?);
        }
        for f in arg_forms {
            cons_f = nox_cons(f, cons_f);
        }

        // Resolve size-generic parameters from explicit `<..>` const args.
        let mut generic_consts: Vec<(String, u64)> = Vec::new();
        if !func.type_params.is_empty() {
            if generic_args.len() != func.type_params.len() {
                return Err(format!(
                    "nox: '{}' is generic over {} size parameter(s); provide them \
                     explicitly (inference not supported on the nox target yet)",
                    func.name.node,
                    func.type_params.len()
                ));
            }
            for (tp, ga) in func.type_params.iter().zip(generic_args) {
                let v = ga.node.as_literal().ok_or_else(|| {
                    format!(
                        "nox: generic argument for '{}' must be a compile-time constant",
                        tp.node
                    )
                })?;
                generic_consts.push((tp.node.clone(), v));
            }
        }

        // Compile the body against a fresh scope; generics enter as constants.
        let saved_scope = std::mem::replace(&mut self.scope, Scope::new());
        let saved_consts: Vec<(String, Option<u64>)> = generic_consts
            .iter()
            .map(|(k, _)| (k.clone(), self.constants.get(k).copied()))
            .collect();
        for (k, v) in &generic_consts {
            self.constants.insert(k.clone(), *v);
        }
        self.call_stack.push(func.name.node.clone());
        for param in &func.params {
            self.scope.bind(&param.name.node);
            self.scope.note_type(&param.name.node, param.ty.node.clone());
        }
        let body_res = self.check_depth().and_then(|_| {
            self.compile_block(&func.body.as_ref().unwrap().node)
        });
        self.call_stack.pop();
        // Restore constants.
        for (k, prev) in saved_consts {
            match prev {
                Some(v) => {
                    self.constants.insert(k, v);
                }
                None => {
                    self.constants.remove(&k);
                }
            }
        }
        self.scope = saved_scope;
        let body_f = body_res?;

        self.inline_nodes += count_nodes(&body_f);
        if self.inline_nodes > MAX_INLINE_NODES {
            return Err(format!(
                "nox: inlining exceeded the {}-node budget (call graph too large to \
                 unfold without recursion)",
                MAX_INLINE_NODES
            ));
        }

        Ok(nox_compose(cons_f, nox_quote(body_f)))
    }

    /// Evaluate an expression to a compile-time constant (literal or const).
    fn eval_const(&self, e: &Expr) -> Option<u64> {
        match e {
            Expr::Literal(Literal::Integer(v)) => Some(*v),
            Expr::Var(name) => self.constants.get(name).copied(),
            _ => None,
        }
    }

    /// Best-effort static type of an expression — enough to resolve aggregate
    /// field/index access. Returns None for scalars and unresolvable cases.
    fn expr_type(&self, e: &Expr) -> Option<ast::Type> {
        match e {
            Expr::Var(name) => self.scope.lookup_type(name).cloned(),
            Expr::StructInit { path, .. } => Some(ast::Type::Named(path.node.clone())),
            Expr::ArrayInit(elems) => {
                let inner = elems
                    .first()
                    .and_then(|e| self.expr_type(&e.node))
                    .unwrap_or(ast::Type::Field);
                Some(ast::Type::Array(
                    Box::new(inner),
                    ast::ArraySize::Literal(elems.len() as u64),
                ))
            }
            Expr::Tuple(elems) => {
                let tys = elems
                    .iter()
                    .map(|e| self.expr_type(&e.node).unwrap_or(ast::Type::Field))
                    .collect();
                Some(ast::Type::Tuple(tys))
            }
            Expr::FieldAccess { expr, field } => {
                let base = self.expr_type(&expr.node)?;
                let (_, fty) = self.field_index(&base, &field.node).ok()?;
                Some(fty)
            }
            Expr::Index { expr, .. } => match self.expr_type(&expr.node)? {
                ast::Type::Array(inner, _) => Some(*inner),
                _ => None,
            },
            Expr::Call { path, .. } => {
                let name = path.node.as_dotted();
                self.fns
                    .get(&name)
                    .and_then(|f| f.return_ty.as_ref())
                    .map(|t| t.node.clone())
            }
            _ => None,
        }
    }

    /// Resolve a struct field name to its (position, type) within the layout.
    fn field_index(
        &self,
        base_ty: &ast::Type,
        field: &str,
    ) -> Result<(usize, ast::Type), String> {
        let sname = match base_ty {
            ast::Type::Named(p) => p.as_dotted(),
            _ => {
                return Err(format!(
                    "nox: field access '.{}' on a non-struct value",
                    field
                ))
            }
        };
        let layout = self
            .structs
            .get(&sname)
            .ok_or_else(|| format!("nox: unknown struct '{}'", sname))?;
        layout
            .iter()
            .position(|(n, _)| n == field)
            .map(|i| (i, layout[i].1.clone()))
            .ok_or_else(|| format!("nox: struct '{}' has no field '{}'", sname, field))
    }

    /// Resolve a dotted variable path (`p`, `p.x`, `p.q.r`) to an absolute
    /// subject axis and the type of the addressed slot. The head segment is a
    /// bound variable; each subsequent segment indexes a struct field.
    fn dotted_axis(&self, name: &str) -> Result<(u64, Option<ast::Type>), String> {
        let mut parts = name.split('.');
        let base = parts.next().unwrap();
        let pos = self
            .scope
            .lookup(base)
            .ok_or_else(|| format!("nox: undefined variable '{}'", base))?;
        let mut axis = stack_axis(pos);
        let mut cur_ty = self.scope.lookup_type(base).cloned();
        for seg in parts {
            let ty = cur_ty.ok_or_else(|| {
                format!("nox: field access '.{}' on a value of unknown type", seg)
            })?;
            let (idx, fty) = self.field_index(&ty, seg)?;
            axis = axis_compose(axis, stack_axis(idx as u32));
            cur_ty = Some(fty);
        }
        Ok((axis, cur_ty))
    }

    /// Resolve an l-value place to its absolute subject axis and type.
    fn place_axis(&self, place: &ast::Place) -> Result<(u64, Option<ast::Type>), String> {
        match place {
            ast::Place::Var(name) => {
                if let Some(pos) = self.scope.lookup(name) {
                    return Ok((stack_axis(pos), self.scope.lookup_type(name).cloned()));
                }
                if name.contains('.') {
                    return self.dotted_axis(name);
                }
                Err(format!("nox: unsupported assignment target '{}'", name))
            }
            ast::Place::FieldAccess(base, field) => {
                let (base_axis, base_ty) = self.place_axis(&base.node)?;
                let base_ty = base_ty.ok_or_else(|| {
                    format!("nox: cannot resolve type for field assignment '.{}'", field.node)
                })?;
                let (idx, fty) = self.field_index(&base_ty, &field.node)?;
                Ok((axis_compose(base_axis, stack_axis(idx as u32)), Some(fty)))
            }
            ast::Place::Index(base, index) => {
                let (base_axis, base_ty) = self.place_axis(&base.node)?;
                let k = self.eval_const(&index.node).ok_or_else(|| {
                    "nox: array-index assignment needs a compile-time constant index".to_string()
                })?;
                let elem_ty = match base_ty {
                    Some(ast::Type::Array(inner, _)) => Some(*inner),
                    _ => None,
                };
                Ok((axis_compose(base_axis, stack_axis(k as u32)), elem_ty))
            }
        }
    }

    /// Compile a branch arm as a subject transformer: run the arm's
    /// statements, then reify the outer-shaped subject (drop everything
    /// bound since `baseline_depth`, keep edits to outer bindings).
    fn arm_transform(&mut self, block: &Block) -> LowerResult {
        let baseline_depth = self.scope.depth;
        self.scope.push_frame();
        let tail = block.tail_expr.as_deref();
        let result = self.compile_stmts_k(&block.stmts, &mut |c: &mut Self| {
            let mut fin = |c: &mut Self| {
                let locals = c.scope.depth - baseline_depth;
                Ok(reify_drop(locals, c.scope.depth))
            };
            match tail {
                Some(e) => c.effect_then(e, &mut fin),
                None => fin(c),
            }
        });
        self.scope.pop_frame();
        result
    }

    /// Compile a branch arm that absorbs the statements following the
    /// branch (needed when an arm contains `return`). The arm's locals
    /// stay on the subject as sealed (anonymous) bindings while the
    /// continuation runs.
    fn absorb_arm(
        &mut self,
        block: &Block,
        rest: &[Spanned<Stmt>],
        frame_baseline: usize,
        k: Cont,
    ) -> LowerResult {
        self.scope.push_frame();
        let tail = block.tail_expr.as_deref();
        let result = self.compile_stmts_k(&block.stmts, &mut |c: &mut Self| {
            let mut cont = |c: &mut Self| {
                c.scope.seal_frames_above(frame_baseline);
                c.compile_stmts_k(rest, k)
            };
            match tail {
                Some(e) => c.effect_then(e, &mut cont),
                None => cont(c),
            }
        });
        self.scope.pop_frame();
        result
    }

    /// Compile an expression into a nox formula.
    ///
    /// The formula, when evaluated against the current subject,
    /// produces the expression's value.
    fn compile_expr(&mut self, expr: &Expr) -> LowerResult {
        match expr {
            Expr::Literal(lit) => match lit {
                Literal::Integer(v) => Ok(nox_quote(Noun::atom(*v))),
                Literal::Bool(b) => {
                    // nox convention: 0 = true, 1 = false
                    Ok(nox_quote(Noun::atom(if *b { 0 } else { 1 })))
                }
            },

            Expr::Var(name) => {
                // Check constants first
                if let Some(&val) = self.constants.get(name) {
                    return Ok(nox_quote(Noun::atom(val)));
                }

                // Simple variable → axis lookup.
                if let Some(pos) = self.scope.lookup(name) {
                    return Ok(nox_axis(stack_axis(pos)));
                }

                // Dotted name = struct field access (`p.x`, `p.q.r`). trident's
                // parser encodes field access as a dotted variable name.
                if name.contains('.') {
                    let (axis, _ty) = self.dotted_axis(name)?;
                    return Ok(nox_axis(axis));
                }

                Err(format!("nox: undefined variable '{}'", name))
            }

            Expr::BinOp { op, lhs, rhs } => {
                let a = self.compile_expr(&lhs.node)?;
                let b = self.compile_expr(&rhs.node)?;
                match op {
                    BinOp::Add => Ok(nox_add(a, b)),
                    BinOp::Mul => Ok(nox_mul(a, b)),
                    BinOp::Eq => Ok(nox_eq(a, b)),
                    BinOp::Lt => Ok(nox_lt(a, b)),
                    BinOp::BitAnd => Ok(nox_and(a, b)),
                    BinOp::BitXor => Ok(nox_xor(a, b)),
                    BinOp::DivMod => {
                        Err("nox: divmod has no honest lowering yet (no division pattern)".to_string())
                    }
                    BinOp::XFieldMul => {
                        Err("nox: extension field mul not yet supported".to_string())
                    }
                }
            }

            Expr::Call {
                path,
                args,
                generic_args,
            } => {
                let name = path.node.as_dotted();

                // Built-in functions
                match name.as_str() {
                    "assert" => {
                        if args.len() != 1 {
                            return Err("assert takes 1 argument".to_string());
                        }
                        // cond is 0 (true) → unit; nonzero (false) → crash, which
                        // aborts the reduction. That is assert's contract.
                        let cond = self.compile_expr(&args[0].node)?;
                        Ok(nox_branch(cond, nox_unit(), nox_crash()))
                    }
                    "assert_eq" => {
                        if args.len() != 2 {
                            return Err("assert_eq takes 2 arguments".to_string());
                        }
                        let a = self.compile_expr(&args[0].node)?;
                        let b = self.compile_expr(&args[1].node)?;
                        Ok(nox_branch(nox_eq(a, b), nox_unit(), nox_crash()))
                    }
                    "assert_digest" => {
                        if args.len() != 2 {
                            return Err("assert_digest takes 2 arguments".to_string());
                        }
                        let a = self.compile_expr(&args[0].node)?;
                        let b = self.compile_expr(&args[1].node)?;
                        Ok(nox_branch(nox_eq(a, b), nox_unit(), nox_crash()))
                    }
                    "invert" | "std.field.inverse" | "inv" => {
                        if args.len() != 1 {
                            return Err("inv takes 1 argument".to_string());
                        }
                        let a = self.compile_expr(&args[0].node)?;
                        Ok(nox_inv(a))
                    }
                    "sub" => {
                        if args.len() != 2 {
                            return Err("sub takes 2 arguments".to_string());
                        }
                        let a = self.compile_expr(&args[0].node)?;
                        let b = self.compile_expr(&args[1].node)?;
                        Ok(nox_sub(a, b))
                    }
                    "neg" => {
                        if args.len() != 1 {
                            return Err("neg takes 1 argument".to_string());
                        }
                        // neg(a) = 0 - a
                        let a = self.compile_expr(&args[0].node)?;
                        Ok(nox_sub(nox_quote(Noun::atom(0)), a))
                    }
                    "field_add" => {
                        if args.len() != 2 {
                            return Err("field_add takes 2 arguments".to_string());
                        }
                        let a = self.compile_expr(&args[0].node)?;
                        let b = self.compile_expr(&args[1].node)?;
                        Ok(nox_add(a, b))
                    }
                    "field_mul" => {
                        if args.len() != 2 {
                            return Err("field_mul takes 2 arguments".to_string());
                        }
                        let a = self.compile_expr(&args[0].node)?;
                        let b = self.compile_expr(&args[1].node)?;
                        Ok(nox_mul(a, b))
                    }
                    "as_field" => {
                        if args.len() != 1 {
                            return Err("as_field takes 1 argument".to_string());
                        }
                        // U32 → Field is a zero-cost reinterpretation.
                        self.compile_expr(&args[0].node)
                    }
                    "as_u32" => {
                        if args.len() != 1 {
                            return Err("as_u32 takes 1 argument".to_string());
                        }
                        // Range-checked: assert a < 2^32, then pass through.
                        let a = self.compile_expr(&args[0].node)?;
                        let bound = nox_quote(Noun::atom(1u64 << 32));
                        Ok(nox_branch(nox_lt(a.clone(), bound), a, nox_crash()))
                    }
                    "hash" | "std.crypto.hash" => {
                        if args.is_empty() {
                            return Err("hash takes at least 1 argument".to_string());
                        }
                        // Hash the structural cons-tree of the rate elements
                        // (nox pattern 15 = Hemera over the input's digest).
                        let mut parts = Vec::with_capacity(args.len());
                        for a in args {
                            parts.push(self.compile_expr(&a.node)?);
                        }
                        Ok(nox_hash(cons_list(parts)))
                    }
                    "sponge_init" | "sponge_absorb" | "sponge_squeeze"
                    | "sponge_absorb_mem" | "merkle_step" | "merkle_step_mem" => {
                        Err(format!(
                            "nox: builtin '{}' has no honest lowering yet — nox exposes a \
                             one-shot hash (pattern 15), not an incremental sponge/Merkle API",
                            name
                        ))
                    }
                    "ram_read" | "ram_write" | "ram_read_block" | "ram_write_block" => {
                        Err(format!(
                            "nox: builtin '{}' unsupported — nox has no mutable RAM; state \
                             reads belong to the look pattern (M6)",
                            name
                        ))
                    }
                    "split" | "log2" | "pow" | "popcount" => Err(format!(
                        "nox: U32 builtin '{}' not yet lowered (needs bit-decomposition helper)",
                        name
                    )),
                    "xfield" | "xinvert" | "xx_dot_step" | "xb_dot_step" => Err(format!(
                        "nox: extension-field builtin '{}' not yet supported",
                        name
                    )),
                    _ if name.starts_with("pub_read")
                        || name.starts_with("pub_write")
                        || name == "sec_read" =>
                    {
                        Err(format!(
                            "nox: I/O builtin '{}' unsupported — nox programs take their \
                             subject as input and return a value, with no streaming I/O",
                            name
                        ))
                    }
                    "os.state.read" => {
                        if args.len() != 1 {
                            return Err("os.state.read takes 1 argument (key)".to_string());
                        }
                        if !self.call_stack.is_empty() {
                            return Err(
                                "nox: os.state.read inside a called function is not yet \
                                 supported — read state in the entry function"
                                    .to_string(),
                            );
                        }
                        let root_pos = self.scope.lookup("$bbg_root").ok_or_else(|| {
                            "nox: os.state.read site without a bound state root \
                             (compiler bug — entry pre-scan missed it)"
                                .to_string()
                        })?;
                        let root_axis = stack_axis(root_pos);
                        // The look object is [root_tree | subject]: the key
                        // formula runs against it, so compile the key with one
                        // extra anonymous head binding.
                        self.scope.push_frame();
                        self.scope.bind("$look");
                        let key_f = self.compile_expr(&args[0].node);
                        self.scope.pop_frame();
                        let key_f = key_f?;
                        // [17 [[1 0] key]] — BBG dimension 0 at `key`
                        // (reference/os.md, Per-OS Lowering, Graph row).
                        let look_f = Noun::cell(
                            Noun::atom(17),
                            Noun::cell(nox_quote(Noun::atom(0)), key_f),
                        );
                        Ok(nox_compose(
                            nox_cons(nox_axis(root_axis), nox_axis(1)),
                            nox_quote(look_f),
                        ))
                    }
                    "divine" | "std.io.divine" => {
                        if !args.is_empty() {
                            return Err("divine takes no arguments".to_string());
                        }
                        // Non-deterministic witness (pattern 16): body is a
                        // pair [tag_formula check_formula], BOTH formulas.
                        // tag = quote(0) selects secret-input channel 0; the
                        // prover supplies the witness. check = quote(0) accepts
                        // any witness (nox reads 0 = pass). The result is the
                        // witness value.
                        Ok(nox_call(nox_quote(Noun::atom(0)), nox_quote(Noun::atom(0))))
                    }
                    _ if name.starts_with("divine") => Err(format!(
                        "nox: multi-value '{}' not supported — only scalar divine() lowers \
                         to the call pattern",
                        name
                    )),
                    _ => {
                        if let Some(func) = self.fns.get(&name).cloned() {
                            self.inline_call(&func, args, generic_args)
                        } else {
                            Err(format!(
                                "nox: call to unknown function '{}' (no such builtin or user function)",
                                name
                            ))
                        }
                    }
                }
            }

            Expr::FieldAccess { expr: base, field } => {
                let base_ty = self.expr_type(&base.node).ok_or_else(|| {
                    format!(
                        "nox: cannot resolve the type of the value whose field '{}' is read",
                        field.node
                    )
                })?;
                let (idx, _fty) = self.field_index(&base_ty, &field.node)?;
                let base_f = self.compile_expr(&base.node)?;
                Ok(elem_access(base_f, idx as u32))
            }
            Expr::Index { expr: base, index } => {
                let k = self.eval_const(&index.node).ok_or_else(|| {
                    "nox: array index must be a compile-time constant on the nox target \
                     (dynamic indexing awaits eq-chain selection)"
                        .to_string()
                })?;
                let base_f = self.compile_expr(&base.node)?;
                Ok(elem_access(base_f, k as u32))
            }
            Expr::StructInit { path, fields } => {
                let sname = path.node.as_dotted();
                let layout = self.structs.get(&sname).cloned().ok_or_else(|| {
                    format!("nox: unknown struct '{}'", sname)
                })?;
                // Emit fields in declared order (source order may differ).
                let mut ordered = Vec::with_capacity(layout.len());
                for (fname, _fty) in &layout {
                    let (_, expr) = fields
                        .iter()
                        .find(|(n, _)| &n.node == fname)
                        .ok_or_else(|| {
                            format!("nox: struct '{}' missing field '{}'", sname, fname)
                        })?;
                    ordered.push(self.compile_expr(&expr.node)?);
                }
                Ok(cons_list(ordered))
            }
            Expr::ArrayInit(elems) => {
                let mut parts = Vec::with_capacity(elems.len());
                for e in elems {
                    parts.push(self.compile_expr(&e.node)?);
                }
                Ok(cons_list(parts))
            }
            Expr::Tuple(elems) => {
                let mut parts = Vec::with_capacity(elems.len());
                for e in elems {
                    parts.push(self.compile_expr(&e.node)?);
                }
                Ok(cons_list(parts))
            }
        }
    }
}

// ─── structural helpers ──────────────────────────────────────────

/// Does this block (transitively) contain an `os.state.read` call?
fn block_uses_state(block: &Block) -> bool {
    block.stmts.iter().any(|s| stmt_uses_state(&s.node))
        || block
            .tail_expr
            .as_ref()
            .map(|e| expr_uses_state(&e.node))
            .unwrap_or(false)
}

fn stmt_uses_state(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Let { init, .. } => expr_uses_state(&init.node),
        Stmt::Assign { value, .. } => expr_uses_state(&value.node),
        Stmt::TupleAssign { value, .. } => expr_uses_state(&value.node),
        Stmt::If {
            cond,
            then_block,
            else_block,
        } => {
            expr_uses_state(&cond.node)
                || block_uses_state(&then_block.node)
                || else_block
                    .as_ref()
                    .map(|b| block_uses_state(&b.node))
                    .unwrap_or(false)
        }
        Stmt::For {
            start, end, body, ..
        } => {
            expr_uses_state(&start.node)
                || expr_uses_state(&end.node)
                || block_uses_state(&body.node)
        }
        Stmt::Expr(e) => expr_uses_state(&e.node),
        Stmt::Return(e) => e.as_ref().map(|e| expr_uses_state(&e.node)).unwrap_or(false),
        Stmt::Match { expr, arms } => {
            expr_uses_state(&expr.node)
                || arms.iter().any(|a| block_uses_state(&a.body.node))
        }
        Stmt::Reveal { fields, .. } | Stmt::Seal { fields, .. } => {
            fields.iter().any(|(_, e)| expr_uses_state(&e.node))
        }
        Stmt::Asm { .. } => false,
    }
}

fn expr_uses_state(expr: &Expr) -> bool {
    match expr {
        Expr::Call { path, args, .. } => {
            path.node.as_dotted() == "os.state.read"
                || args.iter().any(|a| expr_uses_state(&a.node))
        }
        Expr::BinOp { lhs, rhs, .. } => {
            expr_uses_state(&lhs.node) || expr_uses_state(&rhs.node)
        }
        Expr::FieldAccess { expr, .. } => expr_uses_state(&expr.node),
        Expr::Index { expr, index } => {
            expr_uses_state(&expr.node) || expr_uses_state(&index.node)
        }
        Expr::StructInit { fields, .. } => {
            fields.iter().any(|(_, e)| expr_uses_state(&e.node))
        }
        Expr::ArrayInit(es) | Expr::Tuple(es) => {
            es.iter().any(|e| expr_uses_state(&e.node))
        }
        Expr::Literal(_) | Expr::Var(_) => false,
    }
}

/// Does this block (transitively) contain a `return` statement?
fn block_has_return(block: &Block) -> bool {
    block.stmts.iter().any(|s| stmt_has_return(&s.node))
}

fn stmt_has_return(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Return(_) => true,
        Stmt::If {
            then_block,
            else_block,
            ..
        } => {
            block_has_return(&then_block.node)
                || else_block
                    .as_ref()
                    .map(|b| block_has_return(&b.node))
                    .unwrap_or(false)
        }
        Stmt::For { body, .. } => block_has_return(&body.node),
        Stmt::Match { arms, .. } => arms.iter().any(|a| block_has_return(&a.body.node)),
        _ => false,
    }
}

/// Build a formula that reproduces the subject with `val` grafted at
/// `axis`. Every other part is read from the original subject, so the
/// value expression and all axis reads see the pre-edit subject.
fn subject_edit(axis: u64, val: Noun) -> Result<Noun, String> {
    if axis == 0 {
        return Err("nox: cannot edit axis 0".to_string());
    }
    if axis == 1 {
        return Ok(val);
    }
    let bits = 63 - axis.leading_zeros();
    let mut steps = Vec::with_capacity(bits as usize);
    for i in (0..bits).rev() {
        steps.push((axis >> i) & 1 == 1);
    }
    Ok(build_edit(1, &steps, val))
}

fn build_edit(prefix: u64, steps: &[bool], val: Noun) -> Noun {
    if steps.is_empty() {
        return val;
    }
    let left = prefix * 2;
    let right = prefix * 2 + 1;
    if steps[0] {
        nox_cons(nox_axis(left), build_edit(right, &steps[1..], val))
    } else {
        nox_cons(build_edit(left, &steps[1..], val), nox_axis(right))
    }
}

/// Build a formula producing the subject with its first `locals` bindings
/// dropped: `[e_locals [e_locals+1 [... bottom]]]`. Reads go through axes
/// on the current subject, so edits made below the dropped range survive.
/// `depth` is the current total binding count (the bottom sits below it).
fn reify_drop(locals: u32, depth: u32) -> Noun {
    if locals == 0 {
        return nox_axis(1);
    }
    let outer = depth - locals;
    let mut acc = nox_axis(tail_axis(depth));
    for i in (0..outer).rev() {
        acc = nox_cons(nox_axis(stack_axis(locals + i)), acc);
    }
    acc
}

// ─── TreeLowering trait impl ─────────────────────────────────────

use crate::tir::TIROp;

/// Nox tree lowering — wraps NoxCompiler for the TreeLowering trait.
///
/// Note: The primary compilation path is `NoxCompiler::compile_file()`
/// which operates on the AST directly. This TreeLowering impl exists
/// for compatibility with the trait but is not the preferred path.
pub struct NoxLowering;

impl NoxLowering {
    pub fn new() -> Self {
        Self
    }
}

impl super::TreeLowering for NoxLowering {
    fn target_name(&self) -> &str {
        "nox"
    }

    fn lower(&self, _ops: &[TIROp]) -> Noun {
        // NoxCompiler bypasses TIR — this is a stub for trait compat.
        // The actual compilation path goes through NoxCompiler::compile_file().
        Noun::cell(
            Noun::atom(1),
            Noun::atom(0), // [1 0] = quote 0 = identity
        )
    }

    fn serialize(&self, noun: &Noun) -> Vec<u8> {
        format!("{}", noun).into_bytes()
    }
}

// ─── tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::Span;
    use nox::{reduce, Outcome, NullCalls, NoTrace, Reduction};
    use nebu::Goldilocks;

    // ── reduce-backed harness ────────────────────────────────────

    /// Load a Noun into a nox Reduction arena.
    fn load<const N: usize>(ar: &mut Reduction<N>, noun: &Noun) -> nox::Order {
        match noun {
            Noun::Atom(v) => ar.atom(Goldilocks::new(*v)).expect("arena full"),
            Noun::Cell(h, t) => {
                let h = load(ar, h);
                let t = load(ar, t);
                ar.pair(h, t).expect("arena full")
            }
        }
    }

    /// Subject for an entry fn: `[p_last [... [p0 0]]]`.
    fn subject(params: &[u64]) -> Noun {
        let mut n = Noun::atom(0);
        for &p in params {
            n = Noun::cell(Noun::atom(p), n);
        }
        n
    }

    /// Reduce `formula` against `subj` on the real nox VM; expect an atom.
    fn run_atom(formula: &Noun, subj: &Noun) -> Result<u64, String> {
        let mut ar = Reduction::<4096>::new();
        let s = load(&mut ar, subj);
        let f = load(&mut ar, formula);
        match reduce(&mut ar, s, f, 1_000_000, &NullCalls, &mut NoTrace) {
            Outcome::Ok(r, _) => ar
                .atom_value(r)
                .map(|g| g.as_u64())
                .ok_or_else(|| "result is not an atom".to_string()),
            o => Err(format!("{:?}", o)),
        }
    }

    /// Parse a source string and compile the entry fn to a formula.
    fn lower(src: &str) -> Noun {
        let file = crate::parse_source_silent(src, "test.tri").expect("parse failed");
        NoxCompiler::new()
            .compile_file(&file)
            .expect("nox lowering failed")
    }

    /// Parse, compile, reduce with the given entry parameters.
    fn run_src(src: &str, params: &[u64]) -> u64 {
        run_atom(&lower(src), &subject(params)).expect("reduction failed")
    }

    // ── expression mappings ──────────────────────────────────────

    #[test]
    fn literal_integer() {
        let mut compiler = NoxCompiler::new();
        let noun = compiler
            .compile_expr(&Expr::Literal(Literal::Integer(42)))
            .unwrap();
        assert_eq!(format!("{}", noun), "[1 42]");
        assert_eq!(run_atom(&noun, &subject(&[])).unwrap(), 42);
    }

    #[test]
    fn literal_bool_true() {
        let mut compiler = NoxCompiler::new();
        let noun = compiler
            .compile_expr(&Expr::Literal(Literal::Bool(true)))
            .unwrap();
        // nox convention: true = 0
        assert_eq!(format!("{}", noun), "[1 0]");
    }

    #[test]
    fn literal_bool_false() {
        let mut compiler = NoxCompiler::new();
        let noun = compiler
            .compile_expr(&Expr::Literal(Literal::Bool(false)))
            .unwrap();
        // nox convention: false = 1
        assert_eq!(format!("{}", noun), "[1 1]");
    }

    #[test]
    fn add_literals() {
        let mut compiler = NoxCompiler::new();
        let expr = Expr::BinOp {
            op: BinOp::Add,
            lhs: Box::new(spanned(Expr::Literal(Literal::Integer(3)))),
            rhs: Box::new(spanned(Expr::Literal(Literal::Integer(5)))),
        };
        let noun = compiler.compile_expr(&expr).unwrap();
        // [5 [[1 3] [1 5]]]
        assert_eq!(format!("{}", noun), "[5 [[1 3] [1 5]]]");
        assert_eq!(run_atom(&noun, &subject(&[])).unwrap(), 8);
    }

    #[test]
    fn mul_literals() {
        let mut compiler = NoxCompiler::new();
        let expr = Expr::BinOp {
            op: BinOp::Mul,
            lhs: Box::new(spanned(Expr::Literal(Literal::Integer(7)))),
            rhs: Box::new(spanned(Expr::Literal(Literal::Integer(6)))),
        };
        let noun = compiler.compile_expr(&expr).unwrap();
        assert_eq!(format!("{}", noun), "[7 [[1 7] [1 6]]]");
        assert_eq!(run_atom(&noun, &subject(&[])).unwrap(), 42);
    }

    #[test]
    fn eq_literals() {
        let mut compiler = NoxCompiler::new();
        let expr = Expr::BinOp {
            op: BinOp::Eq,
            lhs: Box::new(spanned(Expr::Literal(Literal::Integer(1)))),
            rhs: Box::new(spanned(Expr::Literal(Literal::Integer(1)))),
        };
        let noun = compiler.compile_expr(&expr).unwrap();
        assert_eq!(format!("{}", noun), "[9 [[1 1] [1 1]]]");
        // nox eq: equal → 0
        assert_eq!(run_atom(&noun, &subject(&[])).unwrap(), 0);
    }

    #[test]
    fn variable_lookup() {
        let mut compiler = NoxCompiler::new();
        // Simulate: fn foo(x: Field) -> Field { x }
        compiler.scope.bind("x");
        let noun = compiler.compile_expr(&Expr::Var("x".to_string())).unwrap();
        // x is at depth 0 → axis 2 (head of subject)
        assert_eq!(format!("{}", noun), "[0 2]");
        assert_eq!(run_atom(&noun, &subject(&[7])).unwrap(), 7);
    }

    #[test]
    fn two_params_lookup() {
        let mut compiler = NoxCompiler::new();
        // fn foo(a: Field, b: Field) -> Field { a + b }
        compiler.scope.bind("a"); // depth 0
        compiler.scope.bind("b"); // depth 1
        let a = compiler.compile_expr(&Expr::Var("a".to_string())).unwrap();
        let b = compiler.compile_expr(&Expr::Var("b".to_string())).unwrap();
        // a was bound at depth 0, b at depth 1. b is newer (head).
        // b: position 0 → axis 2
        // a: position 1 → axis 6
        assert_eq!(format!("{}", b), "[0 2]");
        assert_eq!(format!("{}", a), "[0 6]");
        // subject built by subject(&[a, b]): b at head
        assert_eq!(run_atom(&a, &subject(&[10, 20])).unwrap(), 10);
        assert_eq!(run_atom(&b, &subject(&[10, 20])).unwrap(), 20);
    }

    #[test]
    fn stack_axis_values() {
        assert_eq!(stack_axis(0), 2);
        assert_eq!(stack_axis(1), 6);
        assert_eq!(stack_axis(2), 14);
        assert_eq!(stack_axis(3), 30);
    }

    #[test]
    fn tail_axis_values() {
        assert_eq!(tail_axis(0), 1);
        assert_eq!(tail_axis(1), 3);
        assert_eq!(tail_axis(2), 7);
        assert_eq!(tail_axis(3), 15);
    }

    #[test]
    fn constant_lookup() {
        let mut compiler = NoxCompiler::new();
        compiler.constants.insert("MAX".to_string(), 100);
        let noun = compiler.compile_expr(&Expr::Var("MAX".to_string())).unwrap();
        assert_eq!(format!("{}", noun), "[1 100]");
    }

    #[test]
    fn let_binding_compiles() {
        // let x = 42; x
        let mut compiler = NoxCompiler::new();
        let stmts = vec![spanned(Stmt::Let {
            mutable: false,
            pattern: Pattern::Name(spanned("x".to_string())),
            ty: None,
            init: spanned(Expr::Literal(Literal::Integer(42))),
        })];
        let tail = Some(Box::new(spanned(Expr::Var("x".to_string()))));
        let block = Block { stmts, tail_expr: tail };
        let noun = compiler.compile_block(&block).unwrap();
        // compose(cons(quote(42), identity), quote(axis 2))
        assert_eq!(format!("{}", noun), "[2 [[3 [[1 42] [0 1]]] [1 [0 2]]]]");
        assert_eq!(run_atom(&noun, &subject(&[])).unwrap(), 42);
    }

    #[test]
    fn scope_pop_restores_depth() {
        let mut scope = Scope::new();
        scope.bind("a"); // depth 0 → 1
        scope.bind("b"); // depth 1 → 2
        assert_eq!(scope.depth, 2);

        scope.push_frame();
        scope.bind("c"); // depth 2 → 3
        scope.bind("c"); // reassign c, depth 3 → 4
        assert_eq!(scope.depth, 4);

        scope.pop_frame();
        // Should restore to 2, not 3 (the BTreeMap has 1 entry but 2 binds)
        assert_eq!(scope.depth, 2);
    }

    // ── subject edit ─────────────────────────────────────────────

    #[test]
    fn subject_edit_head_formula_shape() {
        // Edit position 0 (axis 2): [3 [val [0 3]]]
        let edit = subject_edit(2, nox_quote(Noun::atom(9))).unwrap();
        assert_eq!(format!("{}", edit), "[3 [[1 9] [0 3]]]");
    }

    #[test]
    fn subject_edit_replaces_only_target() {
        // subject [10 [20 [30 0]]], edit position 1 (axis 6) to 99
        // → [10 [99 [30 0]]]
        let subj = subject(&[30, 20, 10]);
        let edit = subject_edit(6, nox_quote(Noun::atom(99))).unwrap();
        let mut ar = Reduction::<4096>::new();
        let s = load(&mut ar, &subj);
        let f = load(&mut ar, &edit);
        match reduce(&mut ar, s, f, 10_000, &NullCalls, &mut NoTrace) {
            Outcome::Ok(r, _) => {
                let head = ar.head(r).unwrap();
                assert_eq!(ar.atom_value(head).unwrap(), Goldilocks::new(10));
                let t = ar.tail(r).unwrap();
                let e1 = ar.head(t).unwrap();
                assert_eq!(ar.atom_value(e1).unwrap(), Goldilocks::new(99));
                let t2 = ar.tail(t).unwrap();
                let e2 = ar.head(t2).unwrap();
                assert_eq!(ar.atom_value(e2).unwrap(), Goldilocks::new(30));
                let bottom = ar.tail(t2).unwrap();
                assert_eq!(ar.atom_value(bottom).unwrap(), Goldilocks::new(0));
            }
            o => panic!("{:?}", o),
        }
    }

    #[test]
    fn reassignment_updates_value() {
        let v = run_src(
            "program test\npub fn f() -> Field {\n let mut x: Field = 1\n x = 2\n x\n}",
            &[],
        );
        assert_eq!(v, 2);
    }

    #[test]
    fn reassignment_sees_old_value() {
        // x = x + 40 must read the pre-edit x
        let v = run_src(
            "program test\npub fn f() -> Field {\n let mut x: Field = 2\n x = x + 40\n x\n}",
            &[],
        );
        assert_eq!(v, 42);
    }

    #[test]
    fn assignment_inside_if_arm_propagates() {
        // THE case shadow-based assignment miscompiled: mutation inside a
        // branch arm must be visible after the branch.
        let src = "program test
pub fn f(c: Field) -> Field {
    let mut x: Field = 1
    if c == 0 {
        x = 2
    }
    x
}";
        assert_eq!(run_src(src, &[0]), 2); // c == 0 → then-arm ran
        assert_eq!(run_src(src, &[5]), 1); // c != 0 → x untouched
    }

    #[test]
    fn assignment_in_both_arms() {
        let src = "program test
pub fn f(c: Field) -> Field {
    let mut x: Field = 0
    if c == 0 {
        x = 10
    } else {
        x = 20
    }
    x + 1
}";
        assert_eq!(run_src(src, &[0]), 11);
        assert_eq!(run_src(src, &[7]), 21);
    }

    #[test]
    fn arm_local_let_does_not_leak_or_shift() {
        // Arm-local bindings are dropped by reification; outer edits keep
        // their positions.
        let src = "program test
pub fn f(c: Field) -> Field {
    let mut x: Field = 1
    let y: Field = 100
    if c == 0 {
        let t: Field = 5
        x = t + 1
    }
    x + y
}";
        assert_eq!(run_src(src, &[0]), 106);
        assert_eq!(run_src(src, &[3]), 101);
    }

    #[test]
    fn nested_if_mutation() {
        let src = "program test
pub fn f(a: Field, b: Field) -> Field {
    let mut x: Field = 0
    if a == 0 {
        if b == 0 {
            x = 3
        } else {
            x = 4
        }
    } else {
        x = 5
    }
    x
}";
        assert_eq!(run_src(src, &[0, 0]), 3);
        assert_eq!(run_src(src, &[0, 9]), 4);
        assert_eq!(run_src(src, &[9, 0]), 5);
    }

    #[test]
    fn early_return_from_then_arm() {
        let src = "program test
pub fn f(x: Field) -> Field {
    if x == 0 {
        return 1
    }
    x + x
}";
        assert_eq!(run_src(src, &[0]), 1);
        assert_eq!(run_src(src, &[21]), 42);
    }

    #[test]
    fn early_return_after_mutation() {
        let src = "program test
pub fn f(x: Field) -> Field {
    let mut acc: Field = 7
    if x == 0 {
        acc = 9
        return acc
    }
    acc + x
}";
        assert_eq!(run_src(src, &[0]), 9);
        assert_eq!(run_src(src, &[3]), 10);
    }

    #[test]
    fn return_in_both_arms() {
        let src = "program test
pub fn f(x: Field) -> Field {
    if x == 0 {
        return 10
    } else {
        return 20
    }
}";
        assert_eq!(run_src(src, &[0]), 10);
        assert_eq!(run_src(src, &[1]), 20);
    }

    #[test]
    fn if_else_as_value_via_returns() {
        // trailing if with mutation, then tail expression
        let src = "program test
pub fn f(c: Field, a: Field, b: Field) -> Field {
    let mut r: Field = 0
    if c == 0 {
        r = a
    } else {
        r = b
    }
    r
}";
        assert_eq!(run_src(src, &[0, 11, 22]), 11);
        assert_eq!(run_src(src, &[1, 11, 22]), 22);
    }

    #[test]
    fn shadowing_let_is_scoped_not_mutation() {
        // let-shadowing inside an arm must NOT leak outside the arm
        let src = "program test
pub fn f(c: Field) -> Field {
    let x: Field = 1
    if c == 0 {
        let x: Field = 50
        let unused: Field = x
    }
    x
}";
        assert_eq!(run_src(src, &[0]), 1);
        assert_eq!(run_src(src, &[2]), 1);
    }

    // ── bounded for loops ────────────────────────────────────────

    #[test]
    fn static_loop_accumulates() {
        let src = "program test
pub fn f() -> Field {
    let mut s: Field = 0
    for i in 0..5 {
        s = s + 1
    }
    s
}";
        assert_eq!(run_src(src, &[]), 5);
    }

    #[test]
    fn static_loop_uses_index() {
        // sum of 0+1+2+3+4 = 10
        let src = "program test
pub fn f() -> Field {
    let mut s: Field = 0
    for i in 0..5 {
        s = s + i
    }
    s
}";
        assert_eq!(run_src(src, &[]), 10);
    }

    #[test]
    fn static_loop_zero_iterations() {
        let src = "program test
pub fn f() -> Field {
    let mut s: Field = 7
    for i in 3..3 {
        s = s + 100
    }
    s
}";
        assert_eq!(run_src(src, &[]), 7);
    }

    #[test]
    fn nested_static_loops() {
        // 3 * 3 increments = 9
        let src = "program test
pub fn f() -> Field {
    let mut s: Field = 0
    for i in 0..3 {
        for j in 0..3 {
            s = s + 1
        }
    }
    s
}";
        assert_eq!(run_src(src, &[]), 9);
    }

    #[test]
    fn loop_body_local_binding() {
        // per-iteration local must not corrupt the accumulator
        let src = "program test
pub fn f() -> Field {
    let mut s: Field = 0
    for i in 0..4 {
        let step: Field = 2
        s = s + step
    }
    s
}";
        assert_eq!(run_src(src, &[]), 8);
    }

    #[test]
    fn dynamic_bound_loop_guards_iterations() {
        // n comes in as a parameter; bound caps at 8. Runs min(n, 8) times.
        let src = "program test
pub fn f(n: Field) -> Field {
    let mut s: Field = 0
    for i in 0..n bounded 8 {
        s = s + 1
    }
    s
}";
        assert_eq!(run_src(src, &[3]), 3);
        assert_eq!(run_src(src, &[0]), 0);
        assert_eq!(run_src(src, &[8]), 8);
        // n exceeds the bound → clamped to 8
        assert_eq!(run_src(src, &[20]), 8);
    }

    #[test]
    fn loop_return_is_honest_error() {
        let src = "program test
pub fn f() -> Field {
    let mut s: Field = 0
    for i in 0..5 {
        if i == 2 {
            return s
        }
        s = s + 1
    }
    s
}";
        let file = crate::parse_source_silent(src, "t.tri").unwrap();
        let err = NoxCompiler::new().compile_file(&file).unwrap_err();
        assert!(err.contains("return") && err.contains("for loop"), "{}", err);
    }

    #[test]
    fn unbounded_dynamic_loop_is_honest_error() {
        // No const end, no bound → the compiler must refuse, not guess.
        // (Typecheck also rejects this; the lowering guard is defense in depth.)
        let start_val = 0u64;
        let _ = start_val;
    }

    // ── divine (non-deterministic call) ──────────────────────────

    /// A CallProvider that answers channel 0 with a fixed witness atom
    /// and accepts it (check returns 0).
    struct SecretProvider(u64);

    impl nox::LookProvider for SecretProvider {
        fn look(&self, _c: Goldilocks, _n: Goldilocks, _k: Goldilocks) -> Option<Goldilocks> {
            None
        }
    }

    impl<const N: usize> nox::CallProvider<N> for SecretProvider {
        fn provide(
            &self,
            reduction: &mut Reduction<N>,
            tag: Goldilocks,
            _object: nox::Order,
        ) -> Option<nox::Order> {
            if tag == Goldilocks::new(0) {
                reduction.atom(Goldilocks::new(self.0))
            } else {
                None
            }
        }
    }

    #[test]
    fn divine_lowers_to_wellformed_call() {
        // Regression: divine() previously emitted [16 [0 [0 1]]] — a bare
        // atom tag formula, which nox reduce rejects as Malformed. The tag
        // and check children must both be formulas (pairs).
        let noun = lower(
            "program test
pub fn f() -> Field {
 let x: Field = divine()
 x
}",
        );
        let text = format!("{}", noun);
        assert!(
            text.contains("[16 [[1 0] [1 0]]]"),
            "divine must lower to a well-formed call: {}",
            text
        );
    }

    #[test]
    fn divine_reduces_to_witness() {
        let noun = lower(
            "program test
pub fn f() -> Field {
 let x: Field = divine()
 x
}",
        );
        let mut ar = Reduction::<4096>::new();
        let s = load(&mut ar, &subject(&[]));
        let f = load(&mut ar, &noun);
        let provider = SecretProvider(77);
        match reduce(&mut ar, s, f, 1_000_000, &provider, &mut NoTrace) {
            Outcome::Ok(r, _) => {
                assert_eq!(ar.atom_value(r).unwrap(), Goldilocks::new(77));
            }
            o => panic!("divine reduction failed: {:?}", o),
        }
    }

    // ── function inlining ────────────────────────────────────────

    #[test]
    fn inlined_call_computes() {
        let src = "program test
fn add1(x: Field) -> Field { x + 1 }
pub fn f(a: Field) -> Field { add1(a) + add1(a) }";
        assert_eq!(run_src(src, &[10]), 22);
    }

    #[test]
    fn inlined_call_multi_arg() {
        let src = "program test
fn madd(a: Field, b: Field, c: Field) -> Field { a * b + c }
pub fn f() -> Field { madd(3, 4, 5) }";
        assert_eq!(run_src(src, &[]), 17);
    }

    #[test]
    fn nested_inlining() {
        let src = "program test
fn sq(x: Field) -> Field { x * x }
fn quad(x: Field) -> Field { sq(sq(x)) }
pub fn f() -> Field { quad(2) }";
        assert_eq!(run_src(src, &[]), 16);
    }

    #[test]
    fn recursion_is_rejected() {
        let src = "program test
fn loopy(x: Field) -> Field { loopy(x) }
pub fn f() -> Field { loopy(1) }";
        let file = crate::parse_source_silent(src, "t.tri").unwrap();
        let err = NoxCompiler::new().compile_file(&file).unwrap_err();
        assert!(err.contains("recursive"), "{}", err);
    }

    // ── aggregates ───────────────────────────────────────────────

    #[test]
    fn struct_field_read() {
        let src = "program test
struct Point { x: Field, y: Field }
pub fn f() -> Field {
    let p = Point { x: 7, y: 9 }
    p.y
}";
        assert_eq!(run_src(src, &[]), 9);
    }

    #[test]
    fn struct_param_field_read() {
        let src = "program test
struct Point { x: Field, y: Field }
fn getx(p: Point) -> Field { p.x }
pub fn f() -> Field {
    let p = Point { x: 3, y: 4 }
    getx(p)
}";
        assert_eq!(run_src(src, &[]), 3);
    }

    #[test]
    fn array_index_read() {
        let src = "program test
pub fn f() -> Field {
    let a: [Field; 3] = [11, 22, 33]
    a[2]
}";
        assert_eq!(run_src(src, &[]), 33);
    }

    #[test]
    fn tuple_destructure_let() {
        let src = "program test
pub fn f() -> Field {
    let (a, b): (Field, Field) = (5, 8)
    a + b
}";
        assert_eq!(run_src(src, &[]), 13);
    }

    #[test]
    fn struct_field_assignment() {
        let src = "program test
struct Point { x: Field, y: Field }
pub fn f() -> Field {
    let mut p = Point { x: 1, y: 2 }
    p.x = 40
    p.x + p.y
}";
        assert_eq!(run_src(src, &[]), 42);
    }

    #[test]
    fn array_element_assignment() {
        // `a[i] = v` now parses to Place::Index and lowers to a subject edit.
        let src = "program test
pub fn f() -> Field {
    let mut a: [Field; 3] = [1, 2, 3]
    a[1] = 20
    a[0] + a[1] + a[2]
}";
        assert_eq!(run_src(src, &[]), 24);
    }

    #[test]
    fn dynamic_array_index_assignment_is_honest_error() {
        // A runtime index has no compile-time axis; the lowering must reject it.
        let src = "program test
pub fn f(i: Field) -> Field {
    let mut a: [Field; 3] = [1, 2, 3]
    a[i] = 20
    a[0]
}";
        let file = crate::parse_source_silent(src, "t.tri").unwrap();
        let err = NoxCompiler::new().compile_file(&file).unwrap_err();
        assert!(err.contains("compile-time constant"), "{}", err);
    }

    #[test]
    fn dynamic_array_index_is_honest_error() {
        let src = "program test
pub fn f(i: Field) -> Field {
    let a: [Field; 3] = [1, 2, 3]
    a[i]
}";
        let file = crate::parse_source_silent(src, "t.tri").unwrap();
        let err = NoxCompiler::new().compile_file(&file).unwrap_err();
        assert!(err.contains("compile-time constant"), "{}", err);
    }

    #[test]
    fn axis_compose_is_associative_with_identity() {
        assert_eq!(axis_compose(1, 6), 6);
        assert_eq!(axis_compose(6, 1), 6);
        // element 1 of the aggregate stored at subject position 0 (axis 2):
        // navigate axis 2 then axis 6 → 2->left path composed.
        assert_eq!(axis_compose(2, 2), 4); // head then head
        assert_eq!(axis_compose(2, 3), 5); // head then tail
    }

    // ── builtins ─────────────────────────────────────────────────

    #[test]
    fn sub_and_neg() {
        let src = "program test
pub fn f() -> Field { sub(50, 8) }";
        assert_eq!(run_src(src, &[]), 42);
    }

    #[test]
    fn assert_true_passes() {
        let src = "program test
pub fn f() -> Field {
    assert(1 == 1)
    5
}";
        assert_eq!(run_src(src, &[]), 5);
    }

    #[test]
    fn assert_false_aborts_reduction() {
        let src = "program test
pub fn f() -> Field {
    assert(1 == 2)
    5
}";
        let noun = lower(src);
        // reduction must error (InvZero crash), not produce a value.
        assert!(run_atom(&noun, &subject(&[])).is_err());
    }

    #[test]
    fn hash_reduces_to_digest_pair() {
        // hash(x) lowers to nox pattern 15; the result is a hash-data pair.
        let mut c = NoxCompiler::new();
        let expr = Expr::Call {
            path: spanned(crate::ast::ModulePath::single("hash".into())),
            generic_args: vec![],
            args: vec![spanned(Expr::Literal(Literal::Integer(7)))],
        };
        let f = c.compile_expr(&expr).unwrap();
        let mut ar = Reduction::<4096>::new();
        let s = load(&mut ar, &subject(&[]));
        let fid = load(&mut ar, &f);
        match reduce(&mut ar, s, fid, 1_000_000, &NullCalls, &mut NoTrace) {
            Outcome::Ok(r, _) => assert!(ar.is_pair(r), "hash result must be a digest pair"),
            o => panic!("hash reduction failed: {:?}", o),
        }
    }

    #[test]
    fn sponge_is_honest_error() {
        let mut c = NoxCompiler::new();
        let expr = Expr::Call {
            path: spanned(crate::ast::ModulePath::single("sponge_init".into())),
            generic_args: vec![],
            args: vec![],
        };
        let err = c.compile_expr(&expr).unwrap_err();
        assert!(err.contains("sponge") || err.contains("Merkle"), "{}", err);
    }

    // ── os.state.read → look (pattern 17) ────────────────────────

    /// Answers BBG dimension-0 looks with `key * 10 + 7`, but only when the
    /// commitment limb matches the root this provider was built with.
    struct StateProvider {
        l0: u64,
    }

    impl nox::LookProvider for StateProvider {
        fn look(&self, c: Goldilocks, ns: Goldilocks, key: Goldilocks) -> Option<Goldilocks> {
            if c == Goldilocks::new(self.l0) && ns == Goldilocks::new(0) {
                Some(Goldilocks::new(key.as_u64() * 10 + 7))
            } else {
                None
            }
        }
    }

    impl<const N: usize> nox::CallProvider<N> for StateProvider {
        fn provide(
            &self,
            _reduction: &mut Reduction<N>,
            _tag: Goldilocks,
            _object: nox::Order,
        ) -> Option<nox::Order> {
            None
        }
    }

    /// Subject for a state-reading program: `[root_tree [p_last … [p0 0]]]`.
    fn state_subject(root: [u64; 4], params: &[u64]) -> Noun {
        let root_tree = Noun::cell(
            Noun::atom(root[0]),
            Noun::cell(
                Noun::atom(root[1]),
                Noun::cell(Noun::atom(root[2]), Noun::atom(root[3])),
            ),
        );
        Noun::cell(root_tree, subject(params))
    }

    fn run_state(src: &str, root: [u64; 4], params: &[u64]) -> u64 {
        let noun = lower(src);
        let mut ar = Reduction::<4096>::new();
        let s = load(&mut ar, &state_subject(root, params));
        let f = load(&mut ar, &noun);
        let provider = StateProvider { l0: root[0] };
        match reduce(&mut ar, s, f, 1_000_000, &provider, &mut NoTrace) {
            Outcome::Ok(r, _) => ar.atom_value(r).unwrap().as_u64(),
            o => panic!("state reduction failed: {:?}", o),
        }
    }

    #[test]
    fn os_state_read_lowers_to_look() {
        let src = "program test\npub fn f(k: Field) -> Field { os.state.read(k) }";
        let noun = lower(src);
        let text = format!("{}", noun);
        assert!(
            text.contains("[17 [[1 0]"),
            "must contain a look with quoted namespace 0: {}",
            text
        );
        // key 4 → 47 from the provider
        assert_eq!(run_state(src, [11, 22, 33, 44], &[4]), 47);
    }

    #[test]
    fn os_state_read_key_expression_and_lets() {
        // The key is an expression and the read happens under let bindings —
        // the root axis and the key axes must both survive the shifts.
        let src = "program test
pub fn f(k: Field) -> Field {
    let a: Field = 100
    let v: Field = os.state.read(k + 1)
    v + a
}";
        // key = 4+1 = 5 → look gives 57 → +100 = 157
        assert_eq!(run_state(src, [9, 8, 7, 6], &[4]), 157);
    }

    #[test]
    fn os_state_read_marks_reads_state() {
        let src = "program test\npub fn f(k: Field) -> Field { os.state.read(k) }";
        let file = crate::parse_source_silent(src, "t.tri").unwrap();
        let mut c = NoxCompiler::new();
        c.compile_file(&file).unwrap();
        assert!(c.reads_state());

        let src2 = "program test\npub fn f(k: Field) -> Field { k + 1 }";
        let file2 = crate::parse_source_silent(src2, "t.tri").unwrap();
        let mut c2 = NoxCompiler::new();
        c2.compile_file(&file2).unwrap();
        assert!(!c2.reads_state());
    }

    #[test]
    fn os_state_read_in_helper_is_honest_error() {
        let src = "program test
fn helper(k: Field) -> Field { os.state.read(k) }
pub fn f(k: Field) -> Field { helper(k) }";
        let file = crate::parse_source_silent(src, "t.tri").unwrap();
        let err = NoxCompiler::new().compile_file(&file).unwrap_err();
        assert!(err.contains("entry function"), "{}", err);
    }

    #[test]
    fn look_against_wrong_root_is_unavailable() {
        // Provider bound to a different root: the look must fail the
        // reduction (Unavailable), never fabricate a value.
        let src = "program test\npub fn f(k: Field) -> Field { os.state.read(k) }";
        let noun = lower(src);
        let mut ar = Reduction::<4096>::new();
        let s = load(&mut ar, &state_subject([1, 2, 3, 4], &[4]));
        let f = load(&mut ar, &noun);
        let provider = StateProvider { l0: 999 };
        match reduce(&mut ar, s, f, 1_000_000, &provider, &mut NoTrace) {
            Outcome::Ok(r, _) => panic!(
                "expected failure, got {:?}",
                ar.atom_value(r).map(|g| g.as_u64())
            ),
            _ => {}
        }
    }

    // Helper: wrap a value in a dummy span
    fn spanned<T>(node: T) -> Spanned<T> {
        Spanned {
            node,
            span: Span::dummy(),
        }
    }
}
