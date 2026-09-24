// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Type width helpers and struct layout registration/lookup.

use std::collections::BTreeMap;

pub(crate) use crate::ast::display::format_ast_type as format_type_name;
use crate::ast::*;
use crate::target::TerrainConfig;

use super::TIRBuilder;

// ─── Free functions: type helpers ─────────────────────────────────

pub(crate) fn resolve_type_width(ty: &Type, tc: &TerrainConfig) -> u32 {
    match ty {
        Type::Noun => unreachable!("Noun rejected by checked TIR boundary"),
        Type::Field | Type::Bool | Type::U32 => 1,
        Type::XField => tc.xfield_width,
        Type::Digest => tc.digest_width,
        Type::Array(inner, n) => {
            let size = n.as_literal().unwrap_or(0);
            resolve_type_width(inner, tc) * (size as u32)
        }
        Type::Tuple(elems) => elems.iter().map(|t| resolve_type_width(t, tc)).sum(),
        Type::Named(_) => 1,
    }
}

pub(crate) fn resolve_type_width_with_subs(
    ty: &Type,
    subs: &BTreeMap<String, u64>,
    tc: &TerrainConfig,
) -> u32 {
    match ty {
        Type::Noun => unreachable!("Noun rejected by checked TIR boundary"),
        Type::Field | Type::Bool | Type::U32 => 1,
        Type::XField => tc.xfield_width,
        Type::Digest => tc.digest_width,
        Type::Array(inner, n) => {
            let size = n.eval(subs);
            resolve_type_width_with_subs(inner, subs, tc) * (size as u32)
        }
        Type::Tuple(elems) => elems
            .iter()
            .map(|t| resolve_type_width_with_subs(t, subs, tc))
            .sum(),
        Type::Named(_) => 1,
    }
}

// ─── TIRBuilder struct layout methods ──────────────────────────────

impl TIRBuilder {
    /// Resolve source argument order without choosing any machine I/O operation.
    pub(crate) fn entry_leaves(&self, ty: &Type, out: &mut Vec<crate::tir::EntryLeaf>) {
        use crate::tir::EntryLeaf;
        match ty {
            Type::Noun => unreachable!("Noun rejected by checked TIR boundary"),
            Type::Field => out.push(EntryLeaf::Field),
            Type::Bool => out.push(EntryLeaf::Bool),
            Type::U32 => out.push(EntryLeaf::U32),
            Type::Digest => out.extend(vec![
                EntryLeaf::Field;
                self.target_config.digest_width as usize
            ]),
            Type::XField => out.extend(vec![
                EntryLeaf::Field;
                self.target_config.xfield_width as usize
            ]),
            Type::Tuple(elements) => {
                for element in elements {
                    self.entry_leaves(element, out);
                }
            }
            Type::Array(element, size) => {
                let Some(count) = self.array_count(size) else {
                    out.push(EntryLeaf::Unresolved(
                        "array length must resolve to a U32 count".into(),
                    ));
                    return;
                };
                if self.type_width(element) == 0 {
                    // Validate nested extents once, even when there are no words.
                    self.entry_leaves(element, out);
                    return;
                }
                for _ in 0..count {
                    self.entry_leaves(element, out);
                }
            }
            Type::Named(path) => {
                let name = self.qualified_name(&path.0.join("."));
                if let Some(definition) = self.struct_types.get(&name) {
                    for field in &definition.fields {
                        self.entry_leaves(&field.ty.node, out);
                    }
                } else {
                    out.push(EntryLeaf::Unresolved(name));
                }
            }
        }
    }

    /// Register struct field layout from a type annotation.
    pub(crate) fn register_struct_layout_from_type(&mut self, var_name: &str, ty: &Type) {
        if let Type::Named(path) = ty {
            let struct_name = self.qualified_name(&path.0.join("."));
            if let Some(sdef) = self.struct_types.get(&struct_name).cloned() {
                let mut field_map = BTreeMap::new();
                let total: u32 = sdef
                    .fields
                    .iter()
                    .map(|f| self.type_width(&f.ty.node))
                    .sum();
                let mut offset = 0u32;
                for sf in &sdef.fields {
                    let fw = self.type_width(&sf.ty.node);
                    let from_top = total - offset - fw;
                    field_map.insert(sf.name.node.clone(), (from_top, fw));
                    offset += fw;
                }
                self.struct_layouts.insert(var_name.to_string(), field_map);
            }
        }
    }

    /// Look up field offset within a struct variable.
    pub(crate) fn find_field_offset_in_var(
        &self,
        var_name: &str,
        field_name: &str,
    ) -> Option<(u32, u32)> {
        if let Some(offsets) = self.struct_layouts.get(var_name) {
            return offsets.get(field_name).copied();
        }
        None
    }

    /// Resolve field offset for Expr::FieldAccess.
    pub(crate) fn resolve_field_offset(&self, inner: &Expr, field: &str) -> Option<(u32, u32)> {
        if let Some(ty) = self.expr_type(inner) {
            if let Some((field_ty, offset)) = self.field_type_offset(&ty, field) {
                return Some((offset, self.type_width(&field_ty)));
            }
        }
        if let Expr::Var(name) = inner {
            return self.find_field_offset_in_var(name, field);
        }
        None
    }

    /// Resolve a chain of nested field accesses.
    /// Given a base variable and field chain like ["s00", "lo"],
    /// walks through struct layouts and struct type definitions
    /// to compute the combined (offset_from_top, field_width).
    pub(crate) fn resolve_nested_field_offset(
        &self,
        var_name: &str,
        fields: &[&str],
    ) -> Option<(u32, u32)> {
        if fields.is_empty() {
            return None;
        }
        let mut ty = self.var_types.get(var_name)?.clone();
        let mut offset = 0;
        let mut width = 0;
        for field in fields {
            let Type::Named(path) = &ty else {
                return None;
            };
            let name = self.qualified_name(&path.0.join("."));
            let definition = self.struct_types.get(&name)?;
            let position = definition
                .fields
                .iter()
                .position(|f| f.name.node == *field)?;
            let selected = &definition.fields[position];
            // Both offsets are measured from the top of their own aggregate.
            offset += definition.fields[position + 1..]
                .iter()
                .map(|f| self.type_width(&f.ty.node))
                .sum::<u32>();
            width = self.type_width(&selected.ty.node);
            ty = selected.ty.node.clone();
        }
        Some((offset, width))
    }
}
