//! Closed code tables and stable environments for the pure raw ART1 profile.
use super::*;
use ast::{Place, Type};
use layout::{keep, returned};
use plan::{Code, Function, Plan};

mod calls;
mod expr;
mod flow;
mod index;
mod layout;
mod loops;
mod plan;

#[derive(Clone)]
struct Local {
    slot: usize,
    ty: Type,
    // Only literal/global constants and immutable candidate indices count as
    // specialized loop bounds. Ordinary immutable lets remain dynamic.
    range: Option<(u64, u64)>,
}

struct Compiler<'a> {
    owner: &'a mut NoxCompiler,
    plan: &'a Plan,
    function: &'a Function,
    height: u32,
    scopes: Vec<BTreeMap<String, Local>>,
    emitted: &'a mut BTreeMap<Code, Noun>,
}

pub(super) fn compile(
    owner: &mut NoxCompiler,
    entry: &FnDef,
    origins: &BTreeMap<String, (String, Vec<u64>)>,
) -> LowerResult {
    // Reuse the public signature validator; its legacy marshalling is discarded.
    owner.reads_state = owner.state_functions.contains(&entry.name.node);
    owner.adapt_entry(entry, nox_unit())?;
    let plan = Plan::build(owner, entry, origins)?;
    let mut emitted = index::helpers(&plan)?;
    for (name, function) in &plan.functions {
        owner.current_module = name.rsplit_once('.').map(|p| p.0).unwrap_or("").into();
        let mut compiler = Compiler {
            owner,
            plan: &plan,
            function,
            height: layout::height(function.count)?,
            scopes: vec![BTreeMap::new()],
            emitted: &mut emitted,
        };
        for (slot, parameter) in function.definition.params.iter().enumerate() {
            compiler.bind(
                &parameter.name.node,
                Local {
                    slot,
                    ty: parameter.ty.node.clone(),
                    range: None,
                },
            )?;
        }
        let body = function
            .definition
            .body
            .as_ref()
            .ok_or("native function has no body")?;
        let formula = layout::result(compiler.block(&body.node)?);
        compiler
            .emitted
            .insert(Code::Function(name.clone()), formula);
    }
    let mut leaves = vec![Noun::atom(0); plan.codes.len()];
    let mut nodes = 0usize;
    for (code, slot) in &plan.codes {
        let body = emitted
            .remove(code)
            .ok_or_else(|| format!("unemitted native entry {code:?}"))?;
        nodes = nodes.saturating_add(count_nodes(&body));
        if nodes > MAX_INLINE_NODES {
            return Err("native code table exceeds formula node budget".into());
        }
        leaves[*slot] = body;
    }
    let entry_plan = plan
        .functions
        .get(&entry.name.node)
        .ok_or("missing native entry")?;
    let frame = layout::tree(vec![nox_axis(1)], entry_plan.count, true)?;
    let table = layout::tree(leaves, plan.codes.len(), false)?;
    let subject = layout::subject(nox_quote(table), frame);
    Ok(seq(
        subject,
        plan.invoke(&Code::Function(entry.name.node.clone()), nox_axis(1))?,
    ))
}

impl Compiler<'_> {
    fn local(&self, name: &str) -> Option<&Local> {
        self.scopes.iter().rev().find_map(|s| s.get(name))
    }
    fn bind(&mut self, name: &str, local: Local) -> Result<(), String> {
        self.scopes
            .last_mut()
            .ok_or("missing native lexical scope")?
            .insert(name.into(), local);
        Ok(())
    }
    fn slot(&self, slot: usize) -> Noun {
        nox_axis(layout::axis(7, self.height, slot))
    }
    fn set(&self, slot: usize, value: Noun) -> LowerResult {
        layout::set(slot, self.height, value)
    }
    fn range(&self, e: &Expr) -> Option<(u64, u64)> {
        match e {
            Expr::Literal(Literal::Integer(n)) => Some((*n, *n)),
            Expr::Var(n) => match self.local(n) {
                Some(local) => local.range,
                None if n
                    .split_once('.')
                    .is_some_and(|(root, _)| self.local(root).is_some()) =>
                {
                    None
                }
                None => self
                    .owner
                    .constants
                    .get(&self.owner.constant_symbol(n))
                    .map(|n| (*n, *n)),
            },
            _ => None,
        }
    }
    fn constant(&self, e: &Expr) -> Option<u64> {
        self.range(e).and_then(|(a, b)| (a == b).then_some(a))
    }
    fn dotted(&self, name: &str) -> Result<(Noun, Type), String> {
        let mut parts = name.split('.');
        let root = parts.next().ok_or("empty native variable name")?;
        let local = self
            .local(root)
            .ok_or_else(|| format!("undefined native variable {name}"))?;
        let mut value = self.slot(local.slot);
        let mut ty = local.ty.clone();
        for field in parts {
            let (index, next) = self.owner.field_index(&ty, field)?;
            value = elem_access(value, index as u64)?;
            ty = next;
        }
        Ok((value, ty))
    }
}
