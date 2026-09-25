//! One typed, lexical constant environment for checking and both lowerers.
use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{ConstDef, Expr, File, Item, Literal, Type};
use crate::diagnostic::Diagnostic;
use crate::types::Ty;

#[derive(Clone, Debug)]
pub(crate) struct Binding {
    pub owner: String,
    pub name: String,
    pub ty: Ty,
    /// Unreduced integer: dimension checks precede runtime Field emission.
    pub raw: u64,
    pub public: bool,
}

impl Binding {
    pub fn canonical_name(&self) -> String {
        format!("{}.{}", self.owner, self.name)
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Resolved {
    pub locals: BTreeMap<String, Binding>,
    pub visible: BTreeMap<String, Binding>,
}

impl Resolved {
    pub fn raw_values(&self) -> BTreeMap<String, u64> {
        self.visible
            .iter()
            .map(|(name, b)| (name.clone(), b.raw))
            .collect()
    }

    pub fn import_into(&self, visible: &mut BTreeMap<String, Binding>) {
        for binding in self.locals.values().filter(|binding| binding.public) {
            visible.insert(binding.canonical_name(), binding.clone());
            let short = binding.owner.rsplit('.').next().unwrap_or(&binding.owner);
            visible.insert(format!("{short}.{}", binding.name), binding.clone());
        }
    }
}

enum Initializer<'a> {
    Value(Ty, u64),
    Reference(Ty, &'a str),
    Invalid,
}

fn initializer<'a>(def: &'a ConstDef, errors: &mut Vec<Diagnostic>) -> Initializer<'a> {
    let ty = match def.ty.node {
        Type::Field => Ty::Field,
        Type::U32 => Ty::U32,
        _ => {
            let message = if matches!(def.ty.node, Type::Noun) {
                "Noun constants are not supported"
            } else {
                "constant requires Field or U32 type"
            };
            errors.push(Diagnostic::error(message.into(), def.ty.span));
            return Initializer::Invalid;
        }
    };
    match &def.value.node {
        Expr::Literal(Literal::Integer(raw)) => {
            if ty == Ty::U32 && *raw >= (1u64 << 32) {
                errors.push(Diagnostic::error(
                    "U32 constant is out of range".into(),
                    def.value.span,
                ));
                Initializer::Invalid
            } else {
                Initializer::Value(ty, *raw)
            }
        }
        Expr::Var(name) => Initializer::Reference(ty, name),
        _ => {
            errors.push(Diagnostic::error(
                "constant initializer requires an integer literal or constant reference".into(),
                def.value.span,
            ));
            Initializer::Invalid
        }
    }
}

fn evaluate(
    owner: &str,
    def: &ConstDef,
    init: &Initializer<'_>,
    locals: &BTreeMap<String, Option<Binding>>,
    imported: &BTreeMap<String, Binding>,
    errors: &mut Vec<Diagnostic>,
) -> Option<Binding> {
    let (ty, raw) = match init {
        Initializer::Invalid => return None,
        Initializer::Value(ty, raw) => (ty.clone(), *raw),
        Initializer::Reference(ty, name) => {
            let referenced = match locals.get(*name) {
                Some(binding) => binding.as_ref()?,
                None => match imported.get(*name) {
                    Some(binding) => binding,
                    None => {
                        errors.push(Diagnostic::error(
                            format!("unknown constant '{name}'"),
                            def.value.span,
                        ));
                        return None;
                    }
                },
            };
            if *ty != referenced.ty {
                errors.push(Diagnostic::error(
                    format!(
                        "constant type mismatch: expected {ty:?} but got {:?}",
                        referenced.ty
                    ),
                    def.value.span,
                ));
                return None;
            }
            (ty.clone(), referenced.raw)
        }
    };
    Some(Binding {
        owner: owner.into(),
        name: def.name.node.clone(),
        ty,
        raw,
        public: def.is_pub,
    })
}

/// Freeze final active names before signatures; validate replaced declarations too.
/// Iterative dependency traversal keeps long alias chains off the host call stack.
pub(crate) fn resolve(
    file: &File,
    imported: &BTreeMap<String, Binding>,
    flags: &BTreeSet<String>,
) -> Result<Resolved, Vec<Diagnostic>> {
    let mut errors = Vec::new();
    let mut definitions = Vec::new();
    let mut final_indices = BTreeMap::new();
    for item in &file.items {
        if let Item::Const(def) = &item.node {
            if def
                .cfg
                .as_ref()
                .is_some_and(|cfg| !flags.contains(&cfg.node))
            {
                continue;
            }
            final_indices.insert(def.name.node.clone(), definitions.len());
            definitions.push((def, initializer(def, &mut errors)));
        }
    }
    let mut locals: BTreeMap<String, Option<Binding>> = BTreeMap::new();
    for start in final_indices.keys() {
        if locals.contains_key(start) {
            continue;
        }
        let mut stack = vec![start.clone()];
        let mut visiting = BTreeSet::from([start.clone()]);
        while let Some(name) = stack.last().cloned() {
            let (def, init) = &definitions[final_indices[&name]];
            if let Initializer::Reference(_, dependency) = init {
                if final_indices.contains_key(*dependency) && !locals.contains_key(*dependency) {
                    if visiting.insert(dependency.to_string()) {
                        stack.push(dependency.to_string());
                        continue;
                    }
                    errors.push(Diagnostic::error(
                        format!("cyclic constant reference '{dependency}'"),
                        def.value.span,
                    ));
                    locals.insert(name.clone(), None);
                }
            }
            if !locals.contains_key(&name) {
                let binding = evaluate(&file.name.node, def, init, &locals, imported, &mut errors);
                locals.insert(name.clone(), binding);
            }
            visiting.remove(&name);
            stack.pop();
        }
    }
    for (index, (def, init)) in definitions.iter().enumerate() {
        if final_indices[&def.name.node] != index {
            evaluate(&file.name.node, def, init, &locals, imported, &mut errors);
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let locals: BTreeMap<_, _> = locals
        .into_iter()
        .filter_map(|(name, b)| b.map(|b| (name, b)))
        .collect();
    let mut visible = imported.clone();
    visible.extend(locals.clone());
    Ok(Resolved { locals, visible })
}

pub(crate) fn resolve_modules(
    files: &[&File],
    flags: &BTreeSet<String>,
    imported: &BTreeMap<String, Binding>,
) -> Result<Vec<Resolved>, Vec<Diagnostic>> {
    let mut visible = imported.clone();
    let mut modules = Vec::new();
    for file in files {
        let resolved = resolve(file, &visible, flags)?;
        resolved.import_into(&mut visible);
        modules.push(resolved);
    }
    Ok(modules)
}
