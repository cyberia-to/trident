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

use std::collections::BTreeMap;

use crate::ast::{self, BinOp, Block, Expr, FnDef, Item, Literal, Pattern, Stmt};
use crate::span::Spanned;
use super::Noun;

// ─── nox formula constructors (tags 0-16) ────────────────────────

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

// ─── axis arithmetic ─────────────────────────────────────────────

/// Axis address for stack[n] in a right-nested cons list.
/// stack[0] = 2, stack[1] = 6, stack[2] = 14, stack[n] = 2^(n+2) - 2.
fn stack_axis(depth: u32) -> u64 {
    (1u64 << (depth + 2)) - 2
}

// ─── scope ───────────────────────────────────────────────────────

/// Variable scope: maps names to their depth in the subject cons list.
/// Depth 0 = head of subject (most recently bound).
#[derive(Clone, Debug)]
struct Scope {
    /// Stack of (name → bind_depth) frames. Inner vec = one scope level.
    frames: Vec<BTreeMap<String, u32>>,
    /// Total number of cons operations (bindings) on the subject.
    depth: u32,
    /// Number of cons operations per frame (tracks actual binds, not unique names).
    frame_bind_counts: Vec<u32>,
}

impl Scope {
    fn new() -> Self {
        Self {
            frames: vec![BTreeMap::new()],
            depth: 0,
            frame_bind_counts: vec![0],
        }
    }

    fn push_frame(&mut self) {
        self.frames.push(BTreeMap::new());
        self.frame_bind_counts.push(0);
    }

    fn pop_frame(&mut self) {
        self.frames.pop();
        if let Some(count) = self.frame_bind_counts.pop() {
            self.depth -= count;
        }
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
}

// ─── compiler ────────────────────────────────────────────────────

/// Compiles a typed AST directly into nox Noun formulas.
pub struct NoxCompiler {
    scope: Scope,
    /// Resolved constants: name → value.
    constants: BTreeMap<String, u64>,
}

impl NoxCompiler {
    pub fn new() -> Self {
        Self {
            scope: Scope::new(),
            constants: BTreeMap::new(),
        }
    }

    /// Compile an AST file. Returns the Noun formula for the entry function.
    pub fn compile_file(&mut self, file: &ast::File) -> Result<Noun, String> {
        // Collect constants
        for item in &file.items {
            if let Item::Const(c) = &item.node {
                if let Expr::Literal(Literal::Integer(v)) = &c.value.node {
                    self.constants.insert(c.name.node.clone(), *v);
                }
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
    /// as a right-nested cons list: `[param0 [param1 [param2 ... 0]]]`.
    pub fn compile_fn(&mut self, func: &FnDef) -> Result<Noun, String> {
        self.scope = Scope::new();

        // Bind parameters (first param = deepest, last param = head of subject)
        // Parameters are pushed in order, so param[0] is deepest.
        for param in &func.params {
            self.scope.bind(&param.name.node);
        }

        let body = func
            .body
            .as_ref()
            .ok_or_else(|| format!("function {} has no body", func.name.node))?;

        self.compile_block(&body.node)
    }

    /// Compile a block: sequence of statements + optional tail expression.
    fn compile_block(&mut self, block: &Block) -> Result<Noun, String> {
        self.scope.push_frame();

        // Compile statements, building up a chain of let-bindings
        // that modify the subject, with the tail expression at the end.
        let result = self.compile_stmts(&block.stmts, &block.tail_expr)?;

        self.scope.pop_frame();
        Ok(result)
    }

    /// Compile a sequence of statements followed by an optional tail expr.
    fn compile_stmts(
        &mut self,
        stmts: &[Spanned<Stmt>],
        tail: &Option<Box<Spanned<Expr>>>,
    ) -> Result<Noun, String> {
        if stmts.is_empty() {
            return match tail {
                Some(expr) => self.compile_expr(&expr.node),
                None => Ok(nox_quote(Noun::atom(0))), // unit = 0
            };
        }

        let stmt = &stmts[0];
        let rest_stmts = &stmts[1..];

        match &stmt.node {
            Stmt::Let {
                pattern, init, ..
            } => {
                let init_formula = self.compile_expr(&init.node)?;

                match pattern {
                    Pattern::Name(name) => {
                        // Bind variable: cons init onto subject, compile rest
                        self.scope.bind(&name.node);
                        let rest = self.compile_stmts(rest_stmts, tail)?;

                        // [2 [[3 [init [0 1]]] [1 rest]]]
                        // = compose(cons(init, identity), quote(rest))
                        // New subject = [init_result, old_subject]
                        // Then evaluate rest against new subject
                        Ok(nox_compose(
                            nox_cons(init_formula, nox_axis(1)),
                            nox_quote(rest),
                        ))
                    }
                    Pattern::Tuple(names) => {
                        // Tuple destructure: evaluate init, then bind each element.
                        // For now, treat as single binding and extract via axis.
                        // TODO: proper tuple destructuring
                        for name in names {
                            self.scope.bind(&name.node);
                        }
                        let rest = self.compile_stmts(rest_stmts, tail)?;
                        Ok(nox_compose(
                            nox_cons(init_formula, nox_axis(1)),
                            nox_quote(rest),
                        ))
                    }
                }
            }

            Stmt::If {
                cond,
                then_block,
                else_block,
            } => {
                let cond_formula = self.compile_expr(&cond.node)?;
                let then_formula = self.compile_block(&then_block.node)?;

                let else_formula = if let Some(eb) = else_block {
                    self.compile_block(&eb.node)?
                } else {
                    // No else: identity (return current subject value as-is)
                    nox_quote(Noun::atom(0)) // unit
                };

                // nox branch: yes on 0, no on nonzero
                // nox eq returns 0 for true → yes-arm = then_body
                let branch = nox_branch(cond_formula, then_formula, else_formula);

                if rest_stmts.is_empty() && tail.is_none() {
                    Ok(branch)
                } else {
                    // Sequence: branch result becomes new subject for rest.
                    // But branch produces a value, not a subject.
                    // We cons the branch result onto subject, compile rest.
                    self.scope.bind("_if_result");
                    let rest = self.compile_stmts(rest_stmts, tail)?;
                    Ok(nox_compose(
                        nox_cons(branch, nox_axis(1)),
                        nox_quote(rest),
                    ))
                }
            }

            Stmt::Expr(expr) => {
                if rest_stmts.is_empty() && tail.is_none() {
                    // Last statement, no tail — this expr is the result
                    self.compile_expr(&expr.node)
                } else {
                    // Expression statement (side-effect only, discard value).
                    // Just compile the rest — the expr doesn't change subject.
                    self.compile_stmts(rest_stmts, tail)
                }
            }

            Stmt::Return(Some(expr)) => self.compile_expr(&expr.node),
            Stmt::Return(None) => Ok(nox_quote(Noun::atom(0))),

            Stmt::Assign { place, value } => {
                // Mutable assignment — Phase 1: only simple variable reassignment
                if let ast::Place::Var(name) = &place.node {
                    let val = self.compile_expr(&value.node)?;
                    // Rebind: we need to update the subject at the variable's axis.
                    // Simple approach: create new binding that shadows the old one.
                    self.scope.bind(name);
                    let rest = self.compile_stmts(rest_stmts, tail)?;
                    Ok(nox_compose(
                        nox_cons(val, nox_axis(1)),
                        nox_quote(rest),
                    ))
                } else {
                    Err("nox: only simple variable assignment supported".to_string())
                }
            }

            Stmt::For { .. } => Err("nox: for loops not yet supported (Phase 2)".to_string()),
            Stmt::Asm { .. } => Err("nox: inline assembly not supported".to_string()),
            Stmt::Match { .. } => Err("nox: match not yet supported".to_string()),
            Stmt::TupleAssign { .. } => {
                Err("nox: tuple assignment not yet supported".to_string())
            }
            Stmt::Reveal { .. } | Stmt::Seal { .. } => {
                Err("nox: reveal/seal not yet supported".to_string())
            }
        }
    }

    /// Compile an expression into a nox formula.
    ///
    /// The formula, when evaluated against the current subject,
    /// produces the expression's value.
    fn compile_expr(&self, expr: &Expr) -> Result<Noun, String> {
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

                // Variable lookup in scope
                match self.scope.lookup(name) {
                    Some(pos) => Ok(nox_axis(stack_axis(pos))),
                    None => Err(format!("nox: undefined variable '{}'", name)),
                }
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
                        Err("nox: divmod not yet supported".to_string())
                    }
                    BinOp::XFieldMul => {
                        Err("nox: extension field mul not yet supported".to_string())
                    }
                }
            }

            Expr::Call { path, args, .. } => {
                let name = path.node.as_dotted();

                // Built-in functions
                match name.as_str() {
                    "assert" => {
                        if args.len() != 1 {
                            return Err("assert takes 1 argument".to_string());
                        }
                        // Assert: evaluate condition, branch on it.
                        // If false (nonzero in nox) → halt. If true (0) → continue.
                        // For now: just evaluate the condition (nox will halt on nonzero
                        // if we add a crash pattern later). Phase 1: pass through.
                        self.compile_expr(&args[0].node)
                    }
                    "invert" | "std.field.inverse" => {
                        if args.len() != 1 {
                            return Err("invert takes 1 argument".to_string());
                        }
                        let a = self.compile_expr(&args[0].node)?;
                        Ok(nox_inv(a))
                    }
                    "hash" | "std.crypto.hash" => {
                        if args.is_empty() {
                            return Err("hash takes at least 1 argument".to_string());
                        }
                        // Hash single value for now
                        let a = self.compile_expr(&args[0].node)?;
                        Ok(nox_hash(a))
                    }
                    "divine" | "std.io.divine" => {
                        // Non-deterministic hint. Return hint pattern.
                        // [16 [tag subject]]
                        // Phase 1: simple hint with tag 0
                        Ok(Noun::cell(
                            Noun::atom(16),
                            Noun::cell(Noun::atom(0), nox_axis(1)),
                        ))
                    }
                    _ => Err(format!("nox: function call '{}' not yet supported (Phase 2)", name)),
                }
            }

            Expr::FieldAccess { .. } => {
                Err("nox: field access not yet supported".to_string())
            }
            Expr::Index { .. } => {
                Err("nox: indexing not yet supported".to_string())
            }
            Expr::StructInit { .. } => {
                Err("nox: struct init not yet supported".to_string())
            }
            Expr::ArrayInit(_) => {
                Err("nox: array init not yet supported".to_string())
            }
            Expr::Tuple(elems) => {
                // Compile as cons list
                let mut result = nox_quote(Noun::atom(0)); // NIL
                for elem in elems.iter().rev() {
                    let e = self.compile_expr(&elem.node)?;
                    result = nox_cons(e, result);
                }
                Ok(result)
            }
        }
    }
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

    #[test]
    fn literal_integer() {
        let compiler = NoxCompiler::new();
        let noun = compiler
            .compile_expr(&Expr::Literal(Literal::Integer(42)))
            .unwrap();
        assert_eq!(format!("{}", noun), "[1 42]");
    }

    #[test]
    fn literal_bool_true() {
        let compiler = NoxCompiler::new();
        let noun = compiler
            .compile_expr(&Expr::Literal(Literal::Bool(true)))
            .unwrap();
        // nox convention: true = 0
        assert_eq!(format!("{}", noun), "[1 0]");
    }

    #[test]
    fn literal_bool_false() {
        let compiler = NoxCompiler::new();
        let noun = compiler
            .compile_expr(&Expr::Literal(Literal::Bool(false)))
            .unwrap();
        // nox convention: false = 1
        assert_eq!(format!("{}", noun), "[1 1]");
    }

    #[test]
    fn add_literals() {
        let compiler = NoxCompiler::new();
        let expr = Expr::BinOp {
            op: BinOp::Add,
            lhs: Box::new(spanned(Expr::Literal(Literal::Integer(3)))),
            rhs: Box::new(spanned(Expr::Literal(Literal::Integer(5)))),
        };
        let noun = compiler.compile_expr(&expr).unwrap();
        // [5 [[1 3] [1 5]]]
        assert_eq!(format!("{}", noun), "[5 [[1 3] [1 5]]]");
    }

    #[test]
    fn mul_literals() {
        let compiler = NoxCompiler::new();
        let expr = Expr::BinOp {
            op: BinOp::Mul,
            lhs: Box::new(spanned(Expr::Literal(Literal::Integer(7)))),
            rhs: Box::new(spanned(Expr::Literal(Literal::Integer(6)))),
        };
        let noun = compiler.compile_expr(&expr).unwrap();
        assert_eq!(format!("{}", noun), "[7 [[1 7] [1 6]]]");
    }

    #[test]
    fn eq_literals() {
        let compiler = NoxCompiler::new();
        let expr = Expr::BinOp {
            op: BinOp::Eq,
            lhs: Box::new(spanned(Expr::Literal(Literal::Integer(1)))),
            rhs: Box::new(spanned(Expr::Literal(Literal::Integer(1)))),
        };
        let noun = compiler.compile_expr(&expr).unwrap();
        assert_eq!(format!("{}", noun), "[9 [[1 1] [1 1]]]");
    }

    #[test]
    fn variable_lookup() {
        let mut compiler = NoxCompiler::new();
        // Simulate: fn foo(x: Field) -> Field { x }
        compiler.scope.bind("x");
        let noun = compiler.compile_expr(&Expr::Var("x".to_string())).unwrap();
        // x is at depth 0 → axis 2 (head of subject)
        assert_eq!(format!("{}", noun), "[0 2]");
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
    }

    #[test]
    fn stack_axis_values() {
        assert_eq!(stack_axis(0), 2);
        assert_eq!(stack_axis(1), 6);
        assert_eq!(stack_axis(2), 14);
        assert_eq!(stack_axis(3), 30);
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
        // Should be: compose(cons(quote(42), identity), quote(axis 2))
        // [2 [[3 [[1 42] [0 1]]] [1 [0 2]]]]
        assert_eq!(
            format!("{}", noun),
            "[2 [[3 [[1 42] [0 1]]] [1 [0 2]]]]"
        );
    }

    #[test]
    fn reassign_shadows_correctly() {
        // let x = 1; x = 2; x
        let mut compiler = NoxCompiler::new();
        let stmts = vec![
            spanned(Stmt::Let {
                mutable: true,
                pattern: Pattern::Name(spanned("x".to_string())),
                ty: None,
                init: spanned(Expr::Literal(Literal::Integer(1))),
            }),
            spanned(Stmt::Assign {
                place: spanned(ast::Place::Var("x".to_string())),
                value: spanned(Expr::Literal(Literal::Integer(2))),
            }),
        ];
        let tail = Some(Box::new(spanned(Expr::Var("x".to_string()))));
        let block = Block { stmts, tail_expr: tail };
        let noun = compiler.compile_block(&block).unwrap();
        // After let x=1: subject = [1, old]
        // After x=2: subject = [2, [1, old]] — x now at axis 2 (head)
        // Final: axis 2 = the reassigned value
        let output = format!("{}", noun);
        assert!(output.contains("[1 2]"), "should contain quote(2): {}", output);
        assert!(output.contains("[0 2]"), "should lookup head: {}", output);
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

    // Helper: wrap a value in a dummy span
    fn spanned<T>(node: T) -> Spanned<T> {
        Spanned {
            node,
            span: Span::dummy(),
        }
    }
}
