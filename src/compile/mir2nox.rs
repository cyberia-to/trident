//! mir2nox — compile Rust MIR JSON to nox formulas
//!
//! Usage:
//!   nox mir <file.mir.json>              compile all functions
//!   nox mir <file.mir.json> -f relu      compile specific function
//!   rsc --emit=mir-rs --crate-type=lib foo.rs | nox mir -
//!
//! Each function becomes a nox formula. Parameters map to subject positions.
//! Output: one .nox file per function, or all to stdout.

use std::collections::BTreeMap;

/// MIR JSON types (minimal, matching rsc output)
mod mir {
    use serde::Deserialize;

    #[derive(Deserialize, Debug)]
    pub struct Crate {
        pub name: String,
        pub functions: Vec<Function>,
    }

    #[derive(Deserialize, Debug)]
    pub struct Function {
        pub name: String,
        pub params: Vec<Local>,
        pub return_ty: String,
        pub locals: Vec<Local>,
        pub blocks: Vec<Block>,
    }

    #[derive(Deserialize, Debug)]
    pub struct Local {
        pub index: u32,
        pub name: Option<String>,
        pub ty: String,
    }

    #[derive(Deserialize, Debug)]
    pub struct Block {
        pub index: u32,
        pub statements: Vec<Statement>,
        pub terminator: Terminator,
    }

    #[derive(Deserialize, Debug)]
    #[serde(untagged)]
    pub enum Statement {
        Assign { Assign: AssignData },
    }

    #[derive(Deserialize, Debug)]
    pub struct AssignData {
        pub place: Place,
        pub rvalue: Rvalue,
    }

    #[derive(Deserialize, Debug)]
    #[serde(untagged)]
    pub enum Place {
        Local { Local: u32 },
    }

    #[derive(Deserialize, Debug)]
    #[serde(untagged)]
    pub enum Rvalue {
        BinaryOp { BinaryOp: (String, Operand, Operand) },
        Use { Use: Operand },
        UnaryOp { UnaryOp: (String, Operand) },
    }

    #[derive(Deserialize, Debug)]
    #[serde(untagged)]
    pub enum Operand {
        Copy { Copy: Place },
        Move { Move: Place },
        Constant { Constant: ConstVal },
    }

    #[derive(Deserialize, Debug)]
    #[serde(untagged)]
    pub enum ConstVal {
        Scalar { Scalar: u64 },
        Bool { Bool: bool },
    }

    #[derive(Deserialize, Debug)]
    #[serde(untagged)]
    pub enum Terminator {
        Return(String), // "Return"
        Goto { Goto: GotoData },
        SwitchInt { SwitchInt: SwitchData },
        Call { Call: CallData },
    }

    #[derive(Deserialize, Debug)]
    pub struct GotoData {
        pub target: u32,
    }

    #[derive(Deserialize, Debug)]
    pub struct SwitchData {
        pub discriminant: Operand,
        pub targets: Vec<(u64, u32)>,
        pub otherwise: u32,
    }

    #[derive(Deserialize, Debug)]
    pub struct CallData {
        pub func: String,
        pub args: Vec<Operand>,
        pub destination: Place,
        pub target: u32,
    }
}

/// Nox formula as text (matches nox noun text format)
fn nox_atom(v: u64) -> String { format!("{}", v) }
fn nox_cell(a: &str, b: &str) -> String { format!("[{} {}]", a, b) }
fn nox_formula(tag: u64, body: &str) -> String { nox_cell(&nox_atom(tag), body) }
fn nox_axis(addr: u64) -> String { nox_formula(0, &nox_atom(addr)) }
fn nox_quote(val: &str) -> String { nox_formula(1, val) }
fn nox_compose(subj: &str, code: &str) -> String { nox_formula(2, &nox_cell(subj, code)) }
fn nox_cons(a: &str, b: &str) -> String { nox_formula(3, &nox_cell(a, b)) }
fn nox_branch(test: &str, yes: &str, no: &str) -> String {
    nox_formula(4, &nox_cell(test, &nox_cell(yes, no)))
}
fn nox_binop(tag: u64, a: &str, b: &str) -> String { nox_formula(tag, &nox_cell(a, b)) }

/// Compile a MIR function to a nox formula.
pub fn compile_function(func: &mir::Function) -> Result<String, String> {
    // Detect loops: find back-edges in CFG
    let loops = detect_loops(func);

    let mut ctx = CompileCtx::new(func, &loops);
    let formula = ctx.compile_block(0)?;
    Ok(formula)
}

/// A detected loop: header block, set of body blocks, loop-carried locals.
struct LoopInfo {
    header: u32,
    /// Locals that are written in the loop body and read in the header.
    /// These become the loop subject slots.
    carried_locals: Vec<u32>,
    /// Block that exits the loop (target of the condition branch when false)
    exit_block: u32,
    /// Block that enters the loop body (target of condition when true)
    body_entry: u32,
}

/// Find loops by detecting back-edges via DFS.
fn detect_loops(func: &mir::Function) -> Vec<LoopInfo> {
    let mut loops = Vec::new();
    let mut visited = std::collections::BTreeSet::new();
    let mut stack = Vec::new();

    fn successors(block: &mir::Block) -> Vec<u32> {
        match &block.terminator {
            mir::Terminator::Return(_) => vec![],
            mir::Terminator::Goto { Goto: d } => vec![d.target],
            mir::Terminator::SwitchInt { SwitchInt: d } => {
                let mut s: Vec<u32> = d.targets.iter().map(|t| t.1).collect();
                s.push(d.otherwise);
                s
            }
            mir::Terminator::Call { Call: d } => vec![d.target],
        }
    }

    // Simple DFS to find back-edges
    fn dfs(func: &mir::Function, block_idx: u32, visited: &mut std::collections::BTreeSet<u32>,
           stack: &mut Vec<u32>, loops: &mut Vec<LoopInfo>) {
        visited.insert(block_idx);
        stack.push(block_idx);
        if let Some(block) = func.blocks.iter().find(|b| b.index == block_idx) {
            for succ in successors(block) {
                if stack.contains(&succ) {
                    // Back-edge: block_idx → succ (succ is loop header)
                    // Find loop-carried locals: assigned in body blocks
                    let header_pos = stack.iter().position(|&b| b == succ).unwrap();
                    let body_blocks: Vec<u32> = stack[header_pos..].to_vec();
                    let mut carried = std::collections::BTreeSet::new();
                    for &bb_idx in &body_blocks {
                        if let Some(bb) = func.blocks.iter().find(|b| b.index == bb_idx) {
                            for stmt in &bb.statements {
                                let mir::Statement::Assign { Assign: data } = stmt;
                                let mir::Place::Local { Local: l } = data.place;
                                carried.insert(l);
                            }
                            // Call destinations too
                            if let mir::Terminator::Call { Call: d } = &bb.terminator {
                                let mir::Place::Local { Local: l } = d.destination;
                                carried.insert(l);
                            }
                        }
                    }
                    // Find exit/body from header's SwitchInt
                    let header = func.blocks.iter().find(|b| b.index == succ).unwrap();
                    let (exit_block, body_entry) = match &header.terminator {
                        mir::Terminator::SwitchInt { SwitchInt: d } if d.targets.len() == 1 => {
                            (d.targets[0].1, d.otherwise)
                        }
                        _ => continue, // not a simple while loop
                    };
                    // Remove _0 (return) and params from carried
                    let param_locals: std::collections::BTreeSet<u32> =
                        func.params.iter().map(|p| p.index).collect();
                    let carried_locals: Vec<u32> = carried.into_iter()
                        .filter(|l| *l != 0 && !param_locals.contains(l))
                        .collect();
                    loops.push(LoopInfo { header: succ, carried_locals, exit_block, body_entry });
                } else if !visited.contains(&succ) {
                    dfs(func, succ, visited, stack, loops);
                }
            }
        }
        stack.pop();
    }

    dfs(func, 0, &mut visited, &mut stack, &mut loops);
    loops
}

struct CompileCtx<'a> {
    func: &'a mir::Function,
    /// local_index → subject depth (for axis lookup)
    local_depth: BTreeMap<u32, u32>,
    /// Current subject depth
    depth: u32,
    num_params: u32,
    /// Detected loops
    loops: &'a [LoopInfo],
    /// Whether we're currently inside a loop body compilation
    in_loop_body: Option<u32>, // header block idx
}

impl<'a> CompileCtx<'a> {
    fn new(func: &'a mir::Function, loops: &'a [LoopInfo]) -> Self {
        let mut local_depth = BTreeMap::new();
        let num_params = func.params.len() as u32;
        for (i, param) in func.params.iter().enumerate() {
            local_depth.insert(param.index, num_params - 1 - i as u32);
        }
        Self { func, local_depth, depth: num_params, num_params, loops, in_loop_body: None }
    }

    fn push_binding(&mut self, local: u32) -> (BTreeMap<u32, u32>, u32) {
        let saved = (self.local_depth.clone(), self.depth);
        for depth in self.local_depth.values_mut() { *depth += 1; }
        self.local_depth.insert(local, 0);
        self.depth += 1;
        saved
    }

    fn pop_binding(&mut self, saved: (BTreeMap<u32, u32>, u32)) {
        self.local_depth = saved.0;
        self.depth = saved.1;
    }

    /// Axis address for a given depth.
    /// depth 0 → axis 2, depth 1 → axis 6, depth 2 → axis 14, ...
    fn depth_to_axis(depth: u32) -> u64 {
        let mut axis: u64 = 1;
        for _ in 0..=depth {
            axis = axis * 2; // go left (head) at last step
        }
        // Correction: depth 0 = head = axis 2
        // depth 1 = head of tail = go right then left = axis 6
        // Pattern: axis = 2 * (2^depth) - 2
        // Actually: start at 1, then (depth) times go right (tail), then once go left (head)
        // axis = 2 * (2^depth + 2^(depth-1) + ... + 2^1) + 0
        // Simpler: axis = 2*(2^(depth+1) - 2)/2 + 2 ... no.
        // Just compute: axis starts at root(1), take 'depth' right turns, then 1 left.
        // bit pattern: 1 followed by 'depth' 1-bits, then a 0-bit
        // axis 2 = 10 (1 step: left)
        // axis 6 = 110 (2 steps: right, left)
        // axis 14 = 1110 (3 steps: right, right, left)
        // axis 30 = 11110
        // Formula: axis = (2 << depth) - 2 + 2 = (2 << depth)
        // Wait: 2 << 0 = 2 ✓, 2 << 1 = 4 ✗ (should be 6)
        // Let me just compute: 2^(depth+1) + sum of 2^i for i in 0..depth
        // Actually the binary representation: 1 followed by depth 1s followed by 0
        // = (2^(depth+2) - 2) → for depth 0: 2, depth 1: 6, depth 2: 14, depth 3: 30 ✓
        (1u64 << (depth + 2)) - 2
    }

    fn local_axis(&self, local_idx: u32) -> Result<String, String> {
        if let Some(&depth) = self.local_depth.get(&local_idx) {
            Ok(nox_axis(Self::depth_to_axis(depth)))
        } else {
            Err(format!("local {} not in scope", local_idx))
        }
    }

    fn compile_operand(&self, op: &mir::Operand) -> Result<String, String> {
        match op {
            mir::Operand::Copy { Copy: mir::Place::Local { Local: idx } }
            | mir::Operand::Move { Move: mir::Place::Local { Local: idx } } => {
                self.local_axis(*idx)
            }
            mir::Operand::Constant { Constant: mir::ConstVal::Scalar { Scalar: v } } => {
                Ok(nox_quote(&nox_atom(*v)))
            }
            mir::Operand::Constant { Constant: mir::ConstVal::Bool { Bool: v } } => {
                Ok(nox_quote(&nox_atom(if *v { 0 } else { 1 }))) // nox: 0=true
            }
        }
    }

    fn compile_rvalue(&self, rv: &mir::Rvalue) -> Result<String, String> {
        match rv {
            mir::Rvalue::BinaryOp { BinaryOp: (op, a, b) } => {
                let left = self.compile_operand(a)?;
                let right = self.compile_operand(b)?;
                let tag = match op.as_str() {
                    "Add" => 5,
                    "Sub" => 6,
                    "Mul" => 7,
                    // MIR bool convention: 0=false, 1=true
                    // nox eq(a,b): 0 if equal, 1 if not equal
                    // nox lt(a,b): 0 if a<b, 1 if a>=b
                    // MIR Eq: 1 if equal → nox eq returns 0. Need flip.
                    // MIR Lt: 1 if a<b → nox lt returns 0. Need flip.
                    // All comparisons: flip nox result to match MIR bool.
                    "Eq" => {
                        // MIR: 1 if equal. nox eq: 0 if equal. Flip.
                        let eq = nox_binop(9, &left, &right);
                        return Ok(nox_branch(&eq, &nox_quote("1"), &nox_quote("0")));
                    }
                    "Ne" => {
                        // MIR: 1 if not equal. nox eq: 1 if not equal. Same!
                        return Ok(nox_binop(9, &left, &right));
                    }
                    "Lt" => {
                        // MIR: 1 if a<b. nox lt: 0 if a<b. Flip.
                        let lt = nox_binop(10, &left, &right);
                        return Ok(nox_branch(&lt, &nox_quote("1"), &nox_quote("0")));
                    }
                    "Le" => {
                        // MIR: 1 if a<=b. nox lt(b,a): 0 if b<a (a>b), 1 if b>=a (a<=b).
                        // So lt(b,a)=1 when a<=b. Same as MIR Le!
                        return Ok(nox_binop(10, &right, &left));
                    }
                    "Gt" => {
                        // MIR: 1 if a>b. nox lt(b,a): 0 if b<a (a>b). Flip.
                        let lt = nox_binop(10, &right, &left);
                        return Ok(nox_branch(&lt, &nox_quote("1"), &nox_quote("0")));
                    }
                    "Ge" => {
                        // MIR: 1 if a>=b. nox lt(a,b): 1 if a>=b. Same!
                        return Ok(nox_binop(10, &left, &right));
                    }
                    "BitAnd" => 12,
                    "BitXor" => 11,
                    "Shl" => 14,
                    other => return Err(format!("unsupported binop: {}", other)),
                };
                Ok(nox_binop(tag, &left, &right))
            }
            mir::Rvalue::Use { Use: op } => self.compile_operand(op),
            mir::Rvalue::UnaryOp { UnaryOp: (op, val) } => {
                let v = self.compile_operand(val)?;
                match op.as_str() {
                    "Not" => Ok(nox_formula(13, &v)),
                    "Neg" => Ok(nox_binop(6, &nox_quote("0"), &v)), // neg(x) = 0 - x
                    other => Err(format!("unsupported unary: {}", other)),
                }
            }
        }
    }

    fn compile_call(&self, call: &mir::CallData) -> Result<String, String> {
        let fname = &call.func;
        // Map known intrinsics to nox patterns
        if fname.contains("wrapping_add") || fname.contains("unchecked_add") {
            let a = self.compile_operand(&call.args[0])?;
            let b = self.compile_operand(&call.args[1])?;
            return Ok(nox_binop(5, &a, &b));
        }
        if fname.contains("wrapping_sub") || fname.contains("unchecked_sub") {
            let a = self.compile_operand(&call.args[0])?;
            let b = self.compile_operand(&call.args[1])?;
            return Ok(nox_binop(6, &a, &b));
        }
        if fname.contains("wrapping_mul") || fname.contains("unchecked_mul") {
            let a = self.compile_operand(&call.args[0])?;
            let b = self.compile_operand(&call.args[1])?;
            return Ok(nox_binop(7, &a, &b));
        }
        Err(format!("unsupported call: {}", fname))
    }

    /// Compile a block. If it's a loop header, emit self-applying recursion.
    fn compile_block(&mut self, block_idx: u32) -> Result<String, String> {
        // Check if this block is a loop header
        if let Some(loop_info) = self.loops.iter().find(|l| l.header == block_idx) {
            if self.in_loop_body.is_some() {
                return self.emit_back_edge(loop_info);
            }
            return self.emit_loop(loop_info);
        }

        let block = self.func.blocks.iter()
            .find(|b| b.index == block_idx)
            .ok_or_else(|| format!("block {} not found", block_idx))?;

        // Check if this block's Goto leads to a loop header.
        // If so, extract init values from this block's statements and pass to emit_loop.
        if let mir::Terminator::Goto { Goto: goto } = &block.terminator {
            if let Some(loop_info) = self.loops.iter().find(|l| l.header == goto.target) {
                if self.in_loop_body.is_none() {
                    // Extract init values from this block's Assign statements
                    let mut inits = BTreeMap::new();
                    for stmt in &block.statements {
                        let mir::Statement::Assign { Assign: data } = stmt;
                        let mir::Place::Local { Local: dest } = data.place;
                        if let Ok(val) = self.compile_rvalue(&data.rvalue) {
                            inits.insert(dest, val);
                        }
                    }
                    return self.emit_loop_with_inits(loop_info, &inits);
                }
            }
        }

        self.compile_terminator(&block.terminator, &block.statements, 0)
    }

    /// Emit a self-applying loop.
    ///
    /// Subject layout: `[formula [carried_last [... [carried_first [params... 0]]]]]`
    /// - depth 0 = the loop formula itself (for self-application via axis 2)
    /// - depth 1..num_carried = carried locals (last at depth 1, first at depth num_carried)
    /// - depth num_carried+1.. = params
    fn emit_loop(&mut self, info: &LoopInfo) -> Result<String, String> {
        let saved_map = self.local_depth.clone();
        let saved_depth = self.depth;

        let num_carried = info.carried_locals.len() as u32;
        // +1 for the formula slot at depth 0
        let loop_depth = 1 + num_carried + self.num_params;
        self.depth = loop_depth;

        // depth 0 = formula (self-reference)
        // depth 1..num_carried = carried locals (last carried at depth 1)
        for (i, &local) in info.carried_locals.iter().rev().enumerate() {
            self.local_depth.insert(local, 1 + i as u32);
        }
        // Params shift deeper: +1 (formula) + num_carried
        let shift = 1 + num_carried;
        for (i, param) in self.func.params.iter().enumerate() {
            self.local_depth.insert(param.index, shift + (self.num_params - 1 - i as u32));
        }

        // Compile header block as the loop body formula
        self.in_loop_body = Some(info.header);
        let header = self.func.blocks.iter().find(|b| b.index == info.header).unwrap();
        let loop_formula = self.compile_terminator(&header.terminator, &header.statements, 0)?;
        self.in_loop_body = None;

        // Restore
        self.local_depth = saved_map;
        self.depth = saved_depth;

        self.emit_loop_init(info, &loop_formula, &BTreeMap::new())
    }

    /// Emit loop with explicit init values from the pre-header block.
    fn emit_loop_with_inits(&mut self, info: &LoopInfo, inits: &BTreeMap<u32, String>) -> Result<String, String> {
        let saved_map = self.local_depth.clone();
        let saved_depth = self.depth;

        let num_carried = info.carried_locals.len() as u32;
        let loop_depth = 1 + num_carried + self.num_params;
        self.depth = loop_depth;

        for (i, &local) in info.carried_locals.iter().rev().enumerate() {
            self.local_depth.insert(local, 1 + i as u32);
        }
        let shift = 1 + num_carried;
        for (i, param) in self.func.params.iter().enumerate() {
            self.local_depth.insert(param.index, shift + (self.num_params - 1 - i as u32));
        }

        self.in_loop_body = Some(info.header);
        let header = self.func.blocks.iter().find(|b| b.index == info.header).unwrap();
        let loop_formula = self.compile_terminator(&header.terminator, &header.statements, 0)?;
        self.in_loop_body = None;

        self.local_depth = saved_map;
        self.depth = saved_depth;

        self.emit_loop_init(info, &loop_formula, inits)
    }

    fn emit_loop_init(&self, info: &LoopInfo, loop_formula: &str, inits: &BTreeMap<u32, String>) -> Result<String, String> {
        // Build initial subject: [formula [init_carried... [params... 0]]]
        let mut init_subject = nox_axis(1); // outer subject = params
        for &local in info.carried_locals.iter() {
            let init_val = inits.get(&local)
                .cloned()
                .unwrap_or_else(|| nox_quote(&nox_atom(0)));
            init_subject = nox_cons(&init_val, &init_subject);
        }
        init_subject = nox_cons(&nox_quote(loop_formula), &init_subject);

        // compose(init_subject, quote(loop_formula))
        // First iteration: one-shot. Back-edges inside do self-apply via [0 2].
        Ok(nox_compose(&init_subject, &nox_quote(loop_formula)))
    }

    /// Back-edge: build new subject with updated carried locals + formula, self-apply.
    fn emit_back_edge(&self, info: &LoopInfo) -> Result<String, String> {
        // Reconstruct the loop subject: [formula [carried... [params... 0]]]
        // Build from bottom up: nil, then params, then carried, then formula.
        let mut new_subj = nox_quote(&nox_atom(0)); // nil terminator

        // Params in reverse order (first param = deepest)
        for param in self.func.params.iter() {
            let val = self.local_axis(param.index)?;
            new_subj = nox_cons(&val, &new_subj);
        }

        // Carried locals (first = deepest in subject)
        for &local in info.carried_locals.iter() {
            let val = self.local_axis(local)?;
            new_subj = nox_cons(&val, &new_subj);
        }

        // Formula reference: read from current subject's head
        // The formula is always at the same known depth (0) in the loop subject.
        // But in the current body subject (with temporaries), the formula is deeper.
        // We need to find it: it was at depth 0 of the loop subject, now shifted
        // by all body temporaries.
        // Actually: we know the formula's axis in the ORIGINAL loop subject was depth 0.
        // In the loop depth map, depth 0 wasn't assigned to any local — it's implicit.
        // The formula's depth in the CURRENT subject is: (current depth) - (loop depth) + 0
        // Or simpler: we stored the formula at depth 0 of the loop subject. After N
        // push_bindings in the body, it's at depth N.
        // We can compute: formula_current_depth = self.depth - self.num_params - num_carried - 1 ... complex.
        //
        // Easier: the formula is a constant. Just quote it again? No, we don't have it.
        // But we DO have it in the subject. Find its position.
        //
        // The loop subject has formula at depth 0. Body pushed (depth - loop_depth) temps.
        // So formula is at depth (self.depth - (1 + num_carried + self.num_params)).
        // Wait, self.depth was set to loop_depth at start, then push_bindings increased it.
        let num_carried = info.carried_locals.len() as u32;
        let loop_depth = 1 + num_carried + self.num_params;
        let formula_depth = self.depth - loop_depth; // how many temps pushed since loop start
        // Formula was at depth 0 in loop subject, now at depth formula_depth.
        new_subj = nox_cons(&nox_axis(Self::depth_to_axis(formula_depth)), &new_subj);

        // Self-apply: compose(new_subject_builder, [0 2])
        // At this point in the code, we're still inside the CURRENT body subject.
        // compose evaluates both args against current subject:
        // A = new_subj_builder → builds the clean loop subject
        // B = [0 2]? No: [0 2] fetches head of CURRENT subject, which is a body temporary.
        //
        // We need B to eval to the loop formula. The formula is at formula_depth in current subject.
        // B = [0 formula_axis] fetches it.
        let formula_axis = Self::depth_to_axis(formula_depth);
        Ok(nox_compose(&new_subj, &nox_axis(formula_axis)))
    }

    /// Compile statements[stmt_idx..] + terminator into a formula.
    fn compile_terminator(
        &mut self,
        term: &mir::Terminator,
        stmts: &[mir::Statement],
        stmt_idx: usize,
    ) -> Result<String, String> {
        // Process remaining statements first
        if stmt_idx < stmts.len() {
            match &stmts[stmt_idx] {
                mir::Statement::Assign { Assign: data } => {
                    let mir::Place::Local { Local: dest } = data.place;
                    let value = self.compile_rvalue(&data.rvalue)?;

                    // Bind: compose+cons shifts everything +1, new value at depth 0
                    let saved = self.push_binding(dest);
                    let rest = self.compile_terminator(term, stmts, stmt_idx + 1)?;
                    self.pop_binding(saved);

                    let binding = nox_cons(&value, &nox_axis(1));
                    return Ok(nox_compose(&binding, &nox_quote(&rest)));
                }
            }
        }

        // Terminator
        match term {
            mir::Terminator::Return(_) => {
                // Return local 0
                self.local_axis(0)
            }
            mir::Terminator::Goto { Goto: data } => {
                self.compile_block(data.target)
            }
            mir::Terminator::SwitchInt { SwitchInt: data } => {
                let disc = self.compile_operand(&data.discriminant)?;
                if data.targets.len() == 1 && data.targets[0].0 == 0 {
                    let zero_block = data.targets[0].1;  // goto when disc==0
                    let nonzero_block = data.otherwise;   // goto when disc!=0
                    // nox branch: 0=yes, nonzero=no
                    let yes = self.compile_block(zero_block)?;     // disc==0 → yes
                    let no = self.compile_block(nonzero_block)?;   // disc!=0 → no
                    Ok(nox_branch(&disc, &yes, &no))
                } else {
                    Err("complex SwitchInt not supported yet".to_string())
                }
            }
            mir::Terminator::Call { Call: data } => {
                let value = self.compile_call(data)?;
                let mir::Place::Local { Local: dest } = data.destination;

                let saved = self.push_binding(dest);
                let rest = self.compile_block(data.target)?;
                self.pop_binding(saved);

                let binding = nox_cons(&value, &nox_axis(1));
                Ok(nox_compose(&binding, &nox_quote(&rest)))
            }
        }
    }
}

/// Entry point for CLI integration.
pub fn run_mir2nox(args: &[String]) {
    let mut input_path = String::new();
    let mut func_filter: Option<String> = None;
    let mut output_dir: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-f" | "--function" => {
                i += 1;
                if i < args.len() { func_filter = Some(args[i].clone()); }
            }
            "-o" => {
                i += 1;
                if i < args.len() { output_dir = Some(args[i].clone()); }
            }
            "--help" | "-h" => {
                eprintln!("nox mir — compile Rust MIR JSON to nox formulas");
                eprintln!();
                eprintln!("Usage:");
                eprintln!("  nox mir <file.mir.json>          compile all functions");
                eprintln!("  nox mir <file.mir.json> -f relu   compile specific function");
                eprintln!("  rsc --emit=mir-rs foo.rs | nox mir -");
                eprintln!();
                eprintln!("Options:");
                eprintln!("  -f, --function <name>  compile only this function");
                eprintln!("  -o <dir>               output directory for .nox files");
                std::process::exit(0);
            }
            other => { input_path = other.to_string(); }
        }
        i += 1;
    }

    let json_bytes = if input_path == "-" || input_path.is_empty() {
        use std::io::Read;
        let mut buf = String::new();
        if input_path.is_empty() {
            use std::io::IsTerminal;
            if std::io::stdin().is_terminal() {
                eprintln!("error: no input. Run `nox mir --help`.");
                std::process::exit(1);
            }
        }
        std::io::stdin().read_to_string(&mut buf).unwrap_or_else(|e| {
            eprintln!("error reading stdin: {}", e);
            std::process::exit(1);
        });
        buf
    } else {
        std::fs::read_to_string(&input_path).unwrap_or_else(|e| {
            eprintln!("error reading '{}': {}", input_path, e);
            std::process::exit(1);
        })
    };

    let krate: mir::Crate = serde_json::from_str(&json_bytes).unwrap_or_else(|e| {
        eprintln!("error parsing MIR JSON: {}", e);
        std::process::exit(1);
    });

    eprintln!("crate: {} ({} functions)", krate.name, krate.functions.len());

    for func in &krate.functions {
        if let Some(ref filter) = func_filter {
            if !func.name.contains(filter.as_str()) { continue; }
        }

        match compile_function(func) {
            Ok(formula) => {
                let params = func.params.len();
                if let Some(ref dir) = output_dir {
                    let path = format!("{}/{}.nox", dir, func.name);
                    std::fs::write(&path, &formula).unwrap_or_else(|e| {
                        eprintln!("error writing '{}': {}", path, e);
                    });
                    eprintln!("  {} ({} params) → {}", func.name, params, path);
                } else {
                    eprintln!("  {} ({} params):", func.name, params);
                    println!("{}", formula);
                }
            }
            Err(e) => {
                eprintln!("  {} — error: {}", func.name, e);
            }
        }
    }
}
