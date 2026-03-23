// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Type width helpers and struct layout registration/lookup.

use std::collections::BTreeMap;

pub(crate) use crate::ast::display::format_ast_type as format_type_name;
use crate::ast::*;
use crate::span::Spanned;
use crate::target::TerrainConfig;

use super::TIRBuilder;

// ─── Free functions: type helpers ─────────────────────────────────

pub(crate) fn resolve_type_width(ty: &Type, tc: &TerrainConfig) -> u32 {
    match ty {
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
    /// Register struct field layout from a type annotation.
    pub(crate) fn register_struct_layout_from_type(&mut self, var_name: &str, ty: &Type) {
        if let Type::Named(path) = ty {
            let struct_name = path.0.last().map(|s| s.as_str()).unwrap_or("");
            if let Some(sdef) = self.struct_types.get(struct_name).cloned() {
                let mut field_map = BTreeMap::new();
                let total: u32 = sdef
                    .fields
                    .iter()
                    .map(|f| resolve_type_width(&f.ty.node, &self.target_config))
                    .sum();
                let mut offset = 0u32;
                for sf in &sdef.fields {
                    let fw = resolve_type_width(&sf.ty.node, &self.target_config);
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

        // Start with the first field using the variable's layout.
        let first_field = fields[0];
        let (mut offset, mut width) = self.find_field_offset_in_var(var_name, first_field)?;

        // For subsequent fields, we need to look up the struct type
        // of the current field and compute sub-offsets within it.
        let mut current_struct_name = self.find_field_struct_type(var_name, first_field);

        for &field in &fields[1..] {
            let sname = current_struct_name.as_ref()?;
            let sdef = self.struct_types.get(sname)?;

            // Compute offset of `field` within this struct.
            let mut sub_offset = 0u32;
            let total: u32 = sdef
                .fields
                .iter()
                .map(|f| resolve_type_width(&f.ty.node, &self.target_config))
                .sum();
            let mut found = false;
            for sf in &sdef.fields {
                let fw = resolve_type_width(&sf.ty.node, &self.target_config);
                if sf.name.node == field {
                    let from_top = total - sub_offset - fw;
                    // The sub-field is at `from_top` within the parent field.
                    // Adjust: parent's offset already accounts for parent's
                    // position within the whole struct. The sub-field is
                    // within the parent's width window.
                    offset = offset + (width - from_top - fw);
                    width = fw;
                    // Find struct type of this field for next iteration.
                    current_struct_name = if let Type::Named(ref path) = sf.ty.node {
                        path.0.last().cloned()
                    } else {
                        None
                    };
                    found = true;
                    break;
                }
                sub_offset += fw;
            }
            if !found {
                return None;
            }
        }

        Some((offset, width))
    }

    /// Look up the struct type name for a field within a variable's struct.
    fn find_field_struct_type(&self, var_name: &str, field_name: &str) -> Option<String> {
        // First check struct_layouts to confirm the field exists.
        if self.struct_layouts.get(var_name)?.get(field_name).is_none() {
            return None;
        }
        // Find which struct type the variable is, then find the field's type.
        for sdef in self.struct_types.values() {
            let total: u32 = sdef
                .fields
                .iter()
                .map(|f| resolve_type_width(&f.ty.node, &self.target_config))
                .sum();
            // Check if this struct matches the variable's layout.
            if let Some(layout) = self.struct_layouts.get(var_name) {
                let layout_total: u32 = layout.values().map(|(_, w)| w).sum();
                if total != layout_total {
                    continue;
                }
            }
            for sf in &sdef.fields {
                if sf.name.node == field_name {
                    if let Type::Named(ref path) = sf.ty.node {
                        return path.0.last().cloned();
                    }
                    return None;
                }
            }
        }
        None
    }

    /// Compute field widths for a struct init.
    pub(crate) fn compute_struct_field_widths(
        &self,
        ty: &Option<Spanned<Type>>,
        fields: &[(Spanned<String>, Spanned<Expr>)],
    ) -> Vec<u32> {
        if let Some(sp_ty) = ty {
            if let Type::Named(path) = &sp_ty.node {
                if let Some(name) = path.0.last() {
                    if let Some(sdef) = self.struct_types.get(name) {
                        return sdef
                            .fields
                            .iter()
                            .map(|f| resolve_type_width(&f.ty.node, &self.target_config))
                            .collect();
                    }
                }
            }
        }
        vec![1u32; fields.len()]
    }
}
