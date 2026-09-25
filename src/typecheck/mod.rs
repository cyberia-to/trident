// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
mod analysis;
mod block;
mod builtins;
mod capabilities;
pub(crate) mod constants;
mod expr;
mod file;
mod flow;
mod noun;
mod privacy;
mod resolve;
pub(crate) mod specialize;
mod stmt;
mod struct_init;
#[cfg(test)]
mod tests;
pub mod types;

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::*;
use crate::diagnostic::Diagnostic;
use crate::span::{Span, Spanned};
use crate::types::{StructTy, Ty};

/// A function signature for type checking.
#[derive(Clone, Debug)]
pub(super) struct FnSig {
    pub(super) intrinsic: Option<String>,
    pub(super) params: Vec<(String, Ty)>,
    pub(super) return_ty: Ty,
}

/// A generic (size-parameterized) function definition, stored unresolved.
#[derive(Clone, Debug)]
pub struct GenericFnDef {
    pub(super) canonical_name: Option<String>,
    /// Size parameter names, e.g. `["N"]`.
    pub type_params: Vec<String>,
    /// Parameter types as AST types (may contain `ArraySize::Param`).
    pub params: Vec<(String, Type)>,
    pub(super) structs: BTreeMap<String, StructTy>,
    pub(super) constants: BTreeMap<String, u64>,
    /// Return type as AST type (may contain `ArraySize::Param`).
    pub return_ty: Option<Type>,
}

/// A monomorphized instance of a generic function.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MonoInstance {
    /// Original function name.
    pub name: String,
    /// Concrete size values for each type parameter.
    pub size_args: Vec<u64>,
}

impl MonoInstance {
    /// Mangled label: `sum` with N=3 -> `__sum__N3`.
    pub fn mangled_name(&self) -> String {
        let suffix: Vec<String> = self.size_args.iter().map(|n| format!("{}", n)).collect();
        format!("{}__N{}", self.name, suffix.join("_"))
    }
}

/// Variable info in scope.
#[derive(Clone, Debug)]
pub(super) struct VarInfo {
    pub(super) ty: Ty,
    pub(super) mutable: bool,
}

/// An ordinary function's exported signature: (name, params, return_type).
pub type FnExport = (String, Vec<(String, Ty)>, Ty);

/// Exported signatures from a type-checked module.
#[derive(Clone, Debug)]
pub struct ModuleExports {
    pub(crate) resolved_constants: constants::Resolved,
    pub module_name: String,
    pub functions: Vec<FnExport>,
    /// Direct declared intrinsic ownership, separate from transitive requirements.
    pub direct_intrinsics: BTreeMap<String, String>,
    /// Unresolved public size-generic signatures; never encoded as zero-sized ordinary functions.
    pub generic_functions: BTreeMap<String, GenericFnDef>,
    /// Transitive intrinsic requirements, including private helpers for entry checks.
    pub function_requirements: BTreeMap<String, BTreeSet<String>>,
    pub constants: Vec<(String, Ty, u64)>, // (name, ty, value)
    pub structs: Vec<StructTy>,            // exported struct types
    pub warnings: Vec<Diagnostic>,         // non-fatal diagnostics
    /// Unique monomorphized instances of generic functions to emit.
    pub mono_instances: Vec<MonoInstance>,
    /// Checked generic calls keyed by function and callee byte span in this file.
    /// Includes replaced and test bodies, which still require validation.
    pub call_resolutions: BTreeMap<(String, u32, u32), MonoInstance>,
}

pub(crate) struct TypeChecker {
    /// Known function signatures (user-defined + builtins).
    pub(super) functions: BTreeMap<String, FnSig>,
    pub(super) available_intrinsics: BTreeSet<String>,
    pub(super) intrinsic_signatures: BTreeMap<String, FnSig>,
    pub(super) imported_requirements: BTreeMap<String, BTreeSet<String>>,
    /// Variable scopes (stack of scope maps).
    pub(super) scopes: Vec<BTreeMap<String, VarInfo>>,
    /// Known constants (name -> value).
    pub(super) constants: BTreeMap<String, u64>,
    pub(super) constant_types: BTreeMap<String, Ty>,
    pub(super) constant_bindings: BTreeMap<String, constants::Binding>,
    /// Known struct types (name or module.name -> StructTy).
    pub(super) structs: BTreeMap<String, StructTy>,
    pub(super) current_module: String,
    /// Known event types (name -> field list).
    pub(super) events: BTreeMap<String, Vec<(String, Ty)>>,
    /// Accumulated diagnostics.
    pub(super) diagnostics: Vec<Diagnostic>,
    /// Straight-line Field input -> immutable checked U32 binding.
    pub(super) u32_proven: BTreeMap<String, String>,
    pub(super) canonical_as_u32: bool,
    /// Generic (size-parameterized) function definitions.
    pub(super) generic_fns: BTreeMap<String, GenericFnDef>,
    pub(super) current_function: String,
    pub(super) expected_return: Option<Ty>,
    /// Unique monomorphized instances collected during type checking.
    pub(super) mono_instances: Vec<MonoInstance>,
    /// Generic calls keyed by function and callee byte span.
    pub(super) call_resolutions: BTreeMap<(String, u32, u32), MonoInstance>,
    /// Active cfg flags for conditional compilation.
    pub(super) cfg_flags: BTreeSet<String>,
    /// Target VM configuration (digest width, hash rate, field limbs, etc.).
    pub(super) target_config: crate::target::TerrainConfig,
    /// Whether we are currently inside a `#[pure]` function body.
    pub(super) in_pure_fn: bool,
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeChecker {
    pub(crate) fn with_intrinsics(mut self, names: &[String]) -> Self {
        self.available_intrinsics = names.iter().cloned().collect();
        self
    }

    pub(crate) fn new() -> Self {
        Self::with_target(crate::target::TerrainConfig::nox())
    }

    pub(crate) fn with_target(config: crate::target::TerrainConfig) -> Self {
        let mut tc = Self {
            functions: BTreeMap::new(),
            available_intrinsics: config.supported_intrinsics().into_iter().collect(),
            intrinsic_signatures: BTreeMap::new(),
            imported_requirements: BTreeMap::new(),
            scopes: Vec::new(),
            constants: BTreeMap::new(),
            constant_types: BTreeMap::new(),
            constant_bindings: BTreeMap::new(),
            structs: BTreeMap::new(),
            current_module: String::new(),
            events: BTreeMap::new(),
            diagnostics: Vec::new(),
            u32_proven: BTreeMap::new(),
            canonical_as_u32: true,
            generic_fns: BTreeMap::new(),
            current_function: String::new(),
            expected_return: None,
            mono_instances: Vec::new(),
            call_resolutions: BTreeMap::new(),
            cfg_flags: BTreeSet::from(["debug".to_string()]),
            target_config: config,
            in_pure_fn: false,
        };
        tc.register_builtins();
        tc.register_noun_builtins();
        for (name, signature) in &mut tc.functions {
            signature.intrinsic = Some(name.clone());
        }
        tc.intrinsic_signatures = tc.functions.clone();
        tc
    }

    /// Set active cfg flags for conditional compilation.
    pub(crate) fn with_cfg_flags(mut self, flags: BTreeSet<String>) -> Self {
        self.cfg_flags = flags;
        self
    }

    /// Check if an item's cfg attribute is active.
    fn is_cfg_active(&self, cfg: &Option<Spanned<String>>) -> bool {
        match cfg {
            None => true,
            Some(flag) => self.cfg_flags.contains(&flag.node),
        }
    }

    /// Check if a top-level item's cfg is active.
    fn is_item_cfg_active(&self, item: &Item) -> bool {
        match item {
            Item::Fn(f) => self.is_cfg_active(&f.cfg),
            Item::Const(c) => self.is_cfg_active(&c.cfg),
            Item::Struct(s) => self.is_cfg_active(&s.cfg),
            Item::Event(e) => self.is_cfg_active(&e.cfg),
        }
    }

    /// Import exported signatures from another module.
    /// Makes them available as `module_name.fn_name`.
    /// For dotted modules like `std.hash`, also registers under
    /// the short alias `hash.fn_name` so `hash.tip5()` works.
    pub(crate) fn import_module(&mut self, exports: &ModuleExports) {
        // Short alias: last segment of dotted module name
        let short_prefix = exports
            .module_name
            .rsplit('.')
            .next()
            .unwrap_or(&exports.module_name);
        let has_short = short_prefix != exports.module_name;

        for (fn_name, params, return_ty) in &exports.functions {
            let qualified = format!("{}.{}", exports.module_name, fn_name);
            let requirements = exports
                .function_requirements
                .get(fn_name)
                .cloned()
                .unwrap_or_default();
            self.imported_requirements
                .insert(qualified.clone(), requirements.clone());
            if has_short {
                self.imported_requirements
                    .insert(format!("{}.{}", short_prefix, fn_name), requirements);
            }
            let sig = FnSig {
                intrinsic: exports.direct_intrinsics.get(fn_name).cloned(),
                params: params.clone(),
                return_ty: return_ty.clone(),
            };
            self.generic_fns.remove(&qualified);
            self.functions.insert(qualified, sig.clone());
            if has_short {
                let short = format!("{}.{}", short_prefix, fn_name);
                self.generic_fns.remove(&short);
                self.functions.insert(short, sig);
            }
        }
        for (name, definition) in &exports.generic_functions {
            let requirements = exports
                .function_requirements
                .get(name)
                .cloned()
                .unwrap_or_default();
            self.imported_requirements.insert(
                format!("{}.{}", exports.module_name, name),
                requirements.clone(),
            );
            if has_short {
                self.imported_requirements
                    .insert(format!("{}.{}", short_prefix, name), requirements);
            }
            let mut definition = definition.clone();
            definition.canonical_name = Some(format!("{}.{}", exports.module_name, name));
            self.functions
                .remove(&format!("{}.{}", exports.module_name, name));
            self.generic_fns.insert(
                format!("{}.{}", exports.module_name, name),
                definition.clone(),
            );
            if has_short {
                self.functions.remove(&format!("{}.{}", short_prefix, name));
                self.generic_fns
                    .insert(format!("{}.{}", short_prefix, name), definition.clone());
            }
        }
        exports
            .resolved_constants
            .import_into(&mut self.constant_bindings);
        for (const_name, ty, value) in &exports.constants {
            let qualified = format!("{}.{}", exports.module_name, const_name);
            self.constant_types.insert(qualified.clone(), ty.clone());
            self.constants.insert(qualified, *value);
            if has_short {
                let short = format!("{}.{}", short_prefix, const_name);
                self.constant_types.insert(short.clone(), ty.clone());
                self.constants.insert(short, *value);
            }
        }
        for sty in &exports.structs {
            let qualified = format!("{}.{}", exports.module_name, sty.name);
            self.structs.insert(qualified, sty.clone());
            if has_short {
                let short = format!("{}.{}", short_prefix, sty.name);
                self.structs.insert(short, sty.clone());
            }
        }
    }

    // --- Scope management ---

    pub(super) fn push_scope(&mut self) {
        self.u32_proven.clear();
        self.scopes.push(BTreeMap::new());
    }

    pub(super) fn pop_scope(&mut self) {
        self.u32_proven.clear();
        self.scopes.pop();
    }

    pub(super) fn define_var(&mut self, name: &str, ty: Ty, mutable: bool) {
        self.u32_proven
            .retain(|input, binding| input != name && binding != name);
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), VarInfo { ty, mutable });
        }
    }

    pub(super) fn lookup_var(&self, name: &str) -> Option<&VarInfo> {
        for scope in self.scopes.iter().rev() {
            if let Some(info) = scope.get(name) {
                return Some(info);
            }
        }
        None
    }

    // --- Diagnostics ---

    pub(super) fn error(&mut self, msg: String, span: Span) {
        self.diagnostics.push(Diagnostic::error(msg, span));
    }

    pub(super) fn error_with_help(&mut self, msg: String, span: Span, help: String) {
        self.diagnostics
            .push(Diagnostic::error(msg, span).with_help(help));
    }

    pub(super) fn warning(&mut self, msg: String, span: Span) {
        self.diagnostics.push(Diagnostic::warning(msg, span));
    }
}
