//! Deterministic reachable code and storage planning before formula emission.
use super::super::*;
use crate::ast::surface::{self, SurfaceVisitor};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Code {
    Get,
    Edit,
    Ascend,
    Function(String),
    Loop(String, u32, u32),
}
#[derive(Clone)]
pub(super) struct LoopSlots {
    pub index: usize,
    pub remaining: usize,
}
#[derive(Clone)]
pub(super) struct Function {
    pub definition: FnDef,
    pub slots: BTreeMap<(u32, u32), Vec<usize>>,
    pub loops: BTreeMap<(u32, u32), LoopSlots>,
    pub count: usize,
}

pub(super) struct Plan {
    pub functions: BTreeMap<String, Function>,
    pub codes: BTreeMap<Code, usize>,
    pub height: u32,
}

struct Calls {
    names: Vec<String>,
    helpers: BTreeSet<Code>,
}
impl SurfaceVisitor for Calls {
    fn ty(&mut self, _: &ast::Type) {}
    fn call(&mut self, name: &str) {
        self.names.push(name.to_string());
    }
    fn expression(&mut self, expr: &Expr) {
        if matches!(expr, Expr::Index { .. }) {
            self.helpers.insert(Code::Get);
        }
    }
    fn destination(&mut self, place: &ast::Place) {
        if !matches!(place, ast::Place::Var(name) if !name.contains('.')) {
            self.helpers.extend([Code::Edit, Code::Ascend]);
        }
    }
}
impl Plan {
    pub(super) fn build(
        owner: &mut NoxCompiler,
        entry: &FnDef,
        origins: &BTreeMap<String, (String, Vec<u64>)>,
    ) -> Result<Self, String> {
        fn visit(
            owner: &mut NoxCompiler,
            name: &str,
            active: &mut BTreeSet<String>,
            functions: &mut BTreeMap<String, Function>,
            helpers: &mut BTreeSet<Code>,
        ) -> Result<(), String> {
            if active.contains(name) {
                return Err("native source recursion is not supported".into());
            }
            if functions.contains_key(name) {
                return Ok(());
            }
            if active.len() >= 128 || functions.len() >= 4096 {
                return Err("native function planning limit exceeded".into());
            }
            let mut f = owner
                .fns
                .get(name)
                .cloned()
                .ok_or_else(|| format!("missing native function {name}"))?;
            if !f.type_params.is_empty() {
                return Err("raw native generic calls require frontend specialization".into());
            }
            if f.intrinsic.is_some() || f.body.is_none() {
                return Ok(());
            }
            if let Some(b) = &mut f.body {
                ast::normalize_terminal_returns(&mut b.node);
            }
            owner.current_module = name.rsplit_once('.').map(|p| p.0).unwrap_or("").to_string();
            let mut calls = Calls {
                names: Vec::new(),
                helpers: BTreeSet::new(),
            };
            surface::item(&Item::Fn(f.clone()), &mut calls);
            helpers.extend(calls.helpers);
            let callees: Vec<_> = calls
                .names
                .iter()
                .filter_map(|n| owner.function_symbol(n))
                .filter(|n| {
                    owner
                        .fns
                        .get(n)
                        .is_some_and(|f| f.intrinsic.is_none() && f.body.is_some())
                })
                .collect();
            active.insert(name.to_string());
            for callee in callees {
                visit(owner, &callee, active, functions, helpers)?;
            }
            active.remove(name);
            let mut plan = Function {
                count: f.params.len(),
                definition: f.clone(),
                slots: BTreeMap::new(),
                loops: BTreeMap::new(),
            };
            if let Some(body) = &f.body {
                plan.block(&body.node)?;
            }
            if functions.len() >= 4096 {
                return Err("native function planning limit exceeded".into());
            }
            functions.insert(name.to_string(), plan);
            Ok(())
        }
        let mut functions = BTreeMap::new();
        let mut keys = BTreeSet::new();
        visit(
            owner,
            &entry.name.node,
            &mut BTreeSet::new(),
            &mut functions,
            &mut keys,
        )?;
        for (name, function) in &functions {
            keys.insert(Code::Function(name.clone()));
            for &(start, end) in function.loops.keys() {
                keys.insert(Code::Loop(name.clone(), start, end));
            }
        }
        let mut keys: Vec<_> = keys.into_iter().collect();
        keys.sort_by_cached_key(|code| {
            let (name, position) = match code {
                Code::Get => return (0, String::new(), String::new(), Vec::new(), (0, 0, 0)),
                Code::Edit => return (1, String::new(), String::new(), Vec::new(), (0, 0, 0)),
                Code::Ascend => return (2, String::new(), String::new(), Vec::new(), (0, 0, 0)),
                Code::Function(name) => (name, (0, 0, 0)),
                Code::Loop(name, start, end) => (name, (1, *start, *end)),
            };
            let (original, sizes) = origins
                .get(name)
                .cloned()
                .unwrap_or_else(|| (name.clone(), Vec::new()));
            let (module, function) = original.rsplit_once('.').unwrap_or(("", &original));
            (3, module.to_string(), function.to_string(), sizes, position)
        });
        let height = super::layout::height(keys.len())?;
        let codes = keys.into_iter().enumerate().map(|(i, k)| (k, i)).collect();
        Ok(Self {
            functions,
            codes,
            height,
        })
    }
    pub(super) fn lookup(&self, code: &Code) -> LowerResult {
        let slot = *self
            .codes
            .get(code)
            .ok_or_else(|| format!("missing native code entry {code:?}"))?;
        Ok(nox_axis(super::layout::axis(2, self.height, slot)))
    }
    pub(super) fn invoke(&self, code: &Code, subject: Noun) -> LowerResult {
        Ok(nox_compose(subject, self.lookup(code)?))
    }
}
impl Function {
    fn allocate(&mut self, count: usize) -> Result<Vec<usize>, String> {
        let end = self.count.checked_add(count).ok_or("frame size overflow")?;
        super::layout::height(end)?;
        let slots = (self.count..end).collect();
        self.count = end;
        Ok(slots)
    }
    fn block(&mut self, body: &Block) -> Result<(), String> {
        for statement in &body.stmts {
            let key = (statement.span.start, statement.span.end);
            match &statement.node {
                Stmt::Let { pattern, .. } => {
                    let count = match pattern {
                        Pattern::Name(_) => 1,
                        Pattern::Tuple(names) => names.len(),
                    };
                    // One temporary materializes a destructured aggregate once.
                    let slots =
                        self.allocate(count + usize::from(matches!(pattern, Pattern::Tuple(_))))?;
                    if self.slots.insert(key, slots).is_some() {
                        return Err(
                            "native storage planning requires unique statement spans".into()
                        );
                    }
                }
                Stmt::TupleAssign { .. } => {
                    let slots = self.allocate(1)?;
                    if self.slots.insert(key, slots).is_some() {
                        return Err(
                            "native storage planning requires unique statement spans".into()
                        );
                    }
                }
                Stmt::For { body, .. } => {
                    let slots = self.allocate(2)?;
                    if self
                        .loops
                        .insert(
                            key,
                            LoopSlots {
                                index: slots[0],
                                remaining: slots[1],
                            },
                        )
                        .is_some()
                    {
                        return Err("native loop planning requires unique statement spans".into());
                    }
                    self.block(&body.node)?;
                }
                Stmt::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    self.block(&then_block.node)?;
                    if let Some(b) = else_block {
                        self.block(&b.node)?;
                    }
                }
                Stmt::Match { arms, .. } => {
                    for arm in arms {
                        self.block(&arm.body.node)?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}
