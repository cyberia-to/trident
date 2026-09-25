//! A canonical nominal name denotes one resolved layout across declarations.
use super::{constants, resolve::checked_size};
use crate::ast::{ArraySize, File, Item, ModulePath, Type};
use crate::diagnostic::Diagnostic;
use crate::resolve::scope::Scope;
use std::collections::{BTreeMap, BTreeSet};

fn resolved_type(
    ty: &Type,
    aliases: &BTreeMap<String, String>,
    constants: &BTreeMap<String, u64>,
) -> Result<Type, String> {
    Ok(match ty {
        Type::Named(path) => Type::Named(ModulePath(
            aliases
                .get(&path.as_dotted())
                .cloned()
                .unwrap_or_else(|| path.as_dotted())
                .split('.')
                .map(str::to_string)
                .collect(),
        )),
        Type::Array(inner, size) => Type::Array(
            Box::new(resolved_type(inner, aliases, constants)?),
            ArraySize::Literal(checked_size(size, constants)?),
        ),
        Type::Tuple(parts) => Type::Tuple(
            parts
                .iter()
                .map(|part| resolved_type(part, aliases, constants))
                .collect::<Result<_, _>>()?,
        ),
        _ => ty.clone(),
    })
}

/// Compare declarations before name lowering discards their source spellings.
/// Named members compare by defining owner/name: checking the same invariant
/// for every name also fixes all nested layouts, without expanding type graphs.
/// Only repeated names are inspected; normal type resolution still checks
/// source-order availability and target restrictions in its original pass.
pub(super) fn validate(
    file: &File,
    flags: &BTreeSet<String>,
    imported_aliases: &BTreeMap<String, String>,
    constants: &BTreeMap<String, u64>,
) -> Result<(), Vec<Diagnostic>> {
    let definitions: Vec<_> = file
        .items
        .iter()
        .filter_map(|item| match &item.node {
            Item::Struct(definition) if item.node.cfg_active(flags) => Some(definition),
            _ => None,
        })
        .collect();
    let mut aliases = imported_aliases.clone();
    let mut seen = BTreeSet::new();
    let mut repeated = BTreeSet::new();
    for definition in &definitions {
        let name = &definition.name.node;
        aliases.insert(name.clone(), format!("{}.{}", file.name.node, name));
        if !seen.insert(name.clone()) {
            repeated.insert(name.clone());
        }
    }
    let mut layouts = BTreeMap::new();
    for definition in definitions {
        if !repeated.contains(&definition.name.node) {
            continue;
        }
        let fields = definition
            .fields
            .iter()
            .map(|field| {
                resolved_type(&field.ty.node, &aliases, constants)
                    .map(|ty| (field.name.node.clone(), ty, field.is_pub))
                    .map_err(|message| vec![Diagnostic::error(message, field.ty.span)])
            })
            .collect::<Result<Vec<_>, _>>()?;
        if let Some(previous) = layouts.get(&definition.name.node) {
            if previous != &fields {
                return Err(vec![Diagnostic::error(
                    format!(
                        "incompatible redeclaration of struct '{}.{}': ordered fields, \
                         resolved field types and field visibility must remain unchanged",
                        file.name.node, definition.name.node
                    ),
                    definition.name.span,
                )]);
            }
        } else {
            layouts.insert(definition.name.node.clone(), fields);
        }
    }
    Ok(())
}

/// Direct lowerers also receive supplied ASTs. Check layouts before replacing
/// source type names with canonical names; otherwise finite source snapshots
/// can become different layouts or even recursive canonical definitions.
/// This is not another full type-checking pass: supplied AST entrypoints keep
/// their existing contract for bodies, forward references and target support.
pub(crate) fn validate_modules(
    files: &[&File],
    scopes: &[Scope],
    resolved: &[constants::Resolved],
    flags: &BTreeSet<String>,
) -> Result<(), Vec<Diagnostic>> {
    for ((file, scope), constants) in files.iter().zip(scopes).zip(resolved) {
        validate(
            file,
            flags,
            &scope.type_aliases(file, files, flags),
            &constants.raw_values(),
        )?;
    }
    Ok(())
}
