// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! TIRBuilder: lowers a type-checked AST into `Vec<TIROp>`.
//!
//! This is the core of Phase 2 — it replicates the Emitter's AST-walking
//! logic but produces `Vec<TIROp>` instead of `Vec<String>`. The output is
//! target-independent; a `StackLowering` implementation converts it to assembly.
//!
//! Key differences from the Emitter:
//! - No `StackBackend`: instructions are TIROp variants pushed directly.
//! - No `DeferredBlock`: if/else and loops use nested `Vec<TIROp>` bodies
//!   inside structural `TIROp::IfElse`, `TIROp::IfOnly`, and `TIROp::Loop`.
//! - `StackManager` emits typed spill/reload operations directly.

mod assign;
mod call;
mod cleanup;
mod divergence;
mod early_return;
mod expr;
mod functions;
mod helpers;
mod index;
mod layout;
mod match_;
mod native_guard;
mod stmt;
mod types;
#[cfg(test)]
mod tests {
    mod advanced;
    mod basics;
}

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::*;
use crate::target::TerrainConfig;
use crate::tir::stack::StackManager;
use crate::tir::TIROp;
use crate::typecheck::MonoInstance;

use self::layout::{format_type_name, resolve_type_width, resolve_type_width_with_subs};

// ─── TIRBuilder ────────────────────────────────────────────────────

/// Builds IR from a type-checked AST.
pub struct TIRBuilder {
    /// Accumulated IR operations.
    pub(crate) ops: Vec<TIROp>,
    /// Monotonic label counter.
    pub(crate) label_counter: u32,
    /// Stack model: LRU-based manager with automatic RAM spill/reload.
    pub(crate) stack: StackManager,
    /// Struct field layouts: var_name -> { field_name -> (offset_from_top, field_width) }.
    pub(crate) struct_layouts: BTreeMap<String, BTreeMap<String, (u32, u32)>>,
    /// Return widths of user-defined functions.
    pub(crate) fn_return_widths: BTreeMap<String, u32>,
    pub(crate) fn_return_types: BTreeMap<String, Type>,
    pub(crate) var_types: BTreeMap<String, Type>,
    /// Event tags: event name -> sequential integer tag.
    pub(crate) event_tags: BTreeMap<String, u64>,
    /// Event field names in declaration order: event name -> [field_name, ...].
    pub(crate) event_defs: BTreeMap<String, Vec<String>>,
    /// Struct type definitions: struct_name -> StructDef.
    pub(crate) struct_types: BTreeMap<String, StructDef>,
    /// Constants: qualified or short name -> integer value.
    pub(crate) constants: BTreeMap<String, u64>,
    /// Intrinsic map: function name -> intrinsic TASM name.
    pub(crate) intrinsic_map: BTreeMap<String, String>,
    pub(crate) target_intrinsics: BTreeMap<String, (u32, u32)>,
    /// Module alias map: short name -> full module name.
    pub(crate) module_aliases: BTreeMap<String, String>,
    pub(crate) function_aliases: BTreeMap<String, String>,
    /// Monomorphized generic function instances to emit.
    pub(crate) mono_instances: Vec<MonoInstance>,
    /// Generic function AST definitions (name -> FnDef).
    pub(crate) generic_fn_defs: BTreeMap<String, FnDef>,
    /// Current size parameter substitutions during monomorphized emission.
    pub(crate) current_subs: BTreeMap<String, u64>,
    /// Per-call-site resolutions from the type checker.
    pub(crate) call_resolutions: Vec<MonoInstance>,
    /// Index into call_resolutions for the next generic call.
    pub(crate) call_resolution_idx: usize,
    /// Active cfg flags for conditional compilation.
    pub(crate) cfg_flags: BTreeSet<String>,
    /// Target VM configuration.
    pub(crate) target_config: TerrainConfig,
}

impl TIRBuilder {
    pub(crate) fn with_target_intrinsics(mut self, abis: BTreeMap<String, (u32, u32)>) -> Self {
        self.target_intrinsics = abis;
        self
    }

    pub fn new(target_config: TerrainConfig) -> Self {
        let stack = StackManager::with_config(
            // Track the complete abstract operand stack. RAM legalization
            // happens after construction, when every live operand is known.
            u32::MAX,
            target_config.spill_ram_base,
        );
        Self {
            ops: Vec::new(),
            label_counter: 0,
            stack,
            struct_layouts: BTreeMap::new(),
            fn_return_widths: BTreeMap::new(),
            fn_return_types: crate::typecheck::TypeChecker::builtin_return_types(&target_config),
            var_types: BTreeMap::new(),
            event_tags: BTreeMap::new(),
            event_defs: BTreeMap::new(),
            struct_types: BTreeMap::new(),
            constants: BTreeMap::new(),
            intrinsic_map: BTreeMap::new(),
            target_intrinsics: BTreeMap::new(),
            module_aliases: BTreeMap::new(),
            function_aliases: BTreeMap::new(),
            mono_instances: Vec::new(),
            generic_fn_defs: BTreeMap::new(),
            current_subs: BTreeMap::new(),
            call_resolutions: Vec::new(),
            call_resolution_idx: 0,
            cfg_flags: BTreeSet::from(["debug".to_string()]),
            target_config,
        }
    }

    // ── Builder-pattern configuration ─────────────────────────────

    pub fn with_cfg_flags(mut self, flags: BTreeSet<String>) -> Self {
        self.cfg_flags = flags;
        self
    }

    pub fn with_intrinsics(mut self, map: BTreeMap<String, String>) -> Self {
        self.intrinsic_map = map;
        self
    }

    pub fn with_function_aliases(mut self, aliases: BTreeMap<String, String>) -> Self {
        self.function_aliases = aliases;
        self
    }

    pub fn with_module_aliases(mut self, aliases: BTreeMap<String, String>) -> Self {
        self.module_aliases = aliases;
        self
    }

    pub fn with_constants(mut self, constants: BTreeMap<String, u64>) -> Self {
        self.constants.extend(constants);
        self
    }

    pub fn with_mono_instances(mut self, instances: Vec<MonoInstance>) -> Self {
        self.mono_instances = instances;
        self
    }

    pub fn with_call_resolutions(mut self, resolutions: Vec<MonoInstance>) -> Self {
        self.call_resolutions = resolutions;
        self
    }

    // ═══════════════════════════════════════════════════════════════
    // ── Top-level entry: build_file ───────────────────────────────
    // ═══════════════════════════════════════════════════════════════

    pub fn build_file(
        mut self,
        file: &File,
    ) -> Result<Vec<TIROp>, Vec<crate::diagnostic::Diagnostic>> {
        for item in &file.items {
            if !self.is_item_cfg_active(&item.node) {
                continue;
            }
            match &item.node {
                Item::Struct(def) => {
                    self.struct_types.insert(def.name.node.clone(), def.clone());
                }
                Item::Fn(func) => {
                    if let Some(ty) = &func.return_ty {
                        self.fn_return_types
                            .insert(func.name.node.clone(), ty.node.clone());
                    } else {
                        self.fn_return_types
                            .insert(func.name.node.clone(), Type::Tuple(Vec::new()));
                    }
                }
                _ => {}
            }
        }

        // ── Pre-scan: collect constant values ──
        for item in &file.items {
            if !self.is_item_cfg_active(&item.node) {
                continue;
            }
            if let Item::Const(cdef) = &item.node {
                if let Expr::Literal(Literal::Integer(val)) = &cdef.value.node {
                    self.constants.insert(cdef.name.node.clone(), *val);
                }
            }
        }

        // ── Pre-scan: collect intrinsic mappings ──
        for item in &file.items {
            if !self.is_item_cfg_active(&item.node) {
                continue;
            }
            if let Item::Fn(func) = &item.node {
                if let Some(ref intrinsic) = func.intrinsic {
                    let intr_value = if let Some(start) = intrinsic.node.find('(') {
                        let end = intrinsic.node.rfind(')').unwrap_or(intrinsic.node.len());
                        intrinsic.node[start + 1..end].to_string()
                    } else {
                        intrinsic.node.clone()
                    };
                    self.intrinsic_map
                        .insert(func.name.node.clone(), intr_value);
                } else {
                    // Local functions shadow unqualified names imported from SDKs.
                    self.intrinsic_map.remove(&func.name.node);
                }
            }
        }

        self.check_fixed_layout(file)?;

        // ── Pre-scan: collect return widths and detect generic functions ──
        for item in &file.items {
            if !self.is_item_cfg_active(&item.node) {
                continue;
            }
            if let Item::Fn(func) = &item.node {
                if !func.type_params.is_empty() {
                    self.generic_fn_defs
                        .insert(func.name.node.clone(), func.clone());
                } else {
                    let width = func
                        .return_ty
                        .as_ref()
                        .map(|t| self.type_width(&t.node))
                        .unwrap_or(0);
                    self.fn_return_widths.insert(func.name.node.clone(), width);
                }
            }
        }

        // ── Pre-scan: register return widths for monomorphized instances ──
        for inst in &self.mono_instances.clone() {
            if let Some(gdef) = self.generic_fn_defs.get(&inst.name).cloned() {
                let mut subs = BTreeMap::new();
                for (param, val) in gdef.type_params.iter().zip(inst.size_args.iter()) {
                    subs.insert(param.node.clone(), *val);
                }
                let width = gdef
                    .return_ty
                    .as_ref()
                    .map(|t| resolve_type_width_with_subs(&t.node, &subs, &self.target_config))
                    .unwrap_or(0);
                let mangled = inst.mangled_name();
                self.fn_return_widths.insert(mangled, width);
            }
        }

        // ── Pre-scan: collect struct type definitions ──
        for item in &file.items {
            if !self.is_item_cfg_active(&item.node) {
                continue;
            }
            if let Item::Struct(sdef) = &item.node {
                self.struct_types
                    .insert(sdef.name.node.clone(), sdef.clone());
            }
        }

        // ── Pre-scan: assign sequential tags to events ──
        let mut event_tag = 0u64;
        for item in &file.items {
            if !self.is_item_cfg_active(&item.node) {
                continue;
            }
            if let Item::Event(edef) = &item.node {
                self.event_tags.insert(edef.name.node.clone(), event_tag);
                let field_names: Vec<String> =
                    edef.fields.iter().map(|f| f.name.node.clone()).collect();
                self.event_defs.insert(edef.name.node.clone(), field_names);
                event_tag += 1;
            }
        }

        // ── Emit sec ram metadata as comments ──
        for decl in &file.declarations {
            if let Declaration::SecRam(entries) = decl {
                self.ops.push(TIROp::Comment(
                    "sec ram: prover-initialized RAM slots".to_string(),
                ));
                for (addr, ty) in entries {
                    let width = resolve_type_width(&ty.node, &self.target_config);
                    self.ops.push(TIROp::Comment(format!(
                        "ram[{}]: {} ({} field element{})",
                        addr,
                        format_type_name(&ty.node),
                        width,
                        if width == 1 { "" } else { "s" }
                    )));
                }
                // (blank line between sec_ram and functions handled by lowering)
            }
        }

        // ── Program entry point ──
        if file.kind == FileKind::Program {
            if let Some(main) = file.items.iter().find_map(|item| match &item.node {
                Item::Fn(function)
                    if function.name.node == "main" && self.is_item_cfg_active(&item.node) =>
                {
                    Some(function)
                }
                _ => None,
            }) {
                let mut leaves = Vec::new();
                for parameter in &main.params {
                    self.entry_leaves(&parameter.ty.node, &mut leaves);
                }
                if !leaves.is_empty() {
                    self.ops.push(TIROp::EntryParameters(leaves));
                }
            }
            self.ops.push(TIROp::Entry("main".to_string()));
        }

        // ── Emit non-generic, non-test functions ──
        for item in &file.items {
            if !self.is_item_cfg_active(&item.node) {
                continue;
            }
            if let Item::Fn(func) = &item.node {
                if func.type_params.is_empty() && !func.is_test {
                    self.build_fn(func);
                }
            }
        }

        // ── Emit monomorphized copies of generic functions ──
        let instances = self.mono_instances.clone();
        for inst in &instances {
            if let Some(gdef) = self.generic_fn_defs.get(&inst.name).cloned() {
                self.build_mono_fn(&gdef, inst);
            }
        }

        Ok(std::mem::take(&mut self.ops))
    }

    // ═══════════════════════════════════════════════════════════════
    // ── Function emission ─────────────────────────────────────────
    // ═══════════════════════════════════════════════════════════════
}
