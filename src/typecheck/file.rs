//! Register declarations, check bodies and export resolved module signatures.
use super::*;

impl TypeChecker {
    pub(crate) fn check_file(mut self, file: &File) -> Result<ModuleExports, Vec<Diagnostic>> {
        self.current_module = file.name.node.clone();
        let is_std_module = file.name.node.starts_with("std.")
            || file.name.node.starts_with("vm.")
            || file.name.node.starts_with("os.")
            || file.name.node.starts_with("ext.")
            || file.name.node.contains(".ext.");

        let resolved_constants =
            constants::resolve(file, &self.constant_bindings, &self.cfg_flags)?;
        self.constants = resolved_constants.raw_values();
        self.constant_types = resolved_constants
            .visible
            .iter()
            .map(|(name, binding)| (name.clone(), binding.ty.clone()))
            .collect();

        // First pass: register all structs, function signatures, and constants
        for item in &file.items {
            // Skip items excluded by conditional compilation
            if !self.is_item_cfg_active(&item.node) {
                continue;
            }
            match &item.node {
                Item::Struct(sdef) => {
                    let fields: Vec<(String, Ty, bool)> = sdef
                        .fields
                        .iter()
                        .map(|f| (f.name.node.clone(), self.resolve_type(&f.ty.node), f.is_pub))
                        .collect();
                    let sty = StructTy {
                        module: self.current_module.clone(),
                        name: sdef.name.node.clone(),
                        fields,
                    };
                    self.structs.insert(sdef.name.node.clone(), sty);
                }
                Item::Fn(func) => {
                    let mut size_names = BTreeSet::new();
                    for parameter in &func.type_params {
                        if !size_names.insert(&parameter.node) {
                            self.error(
                                format!("duplicate size parameter '{}'", parameter.node),
                                parameter.span,
                            );
                        }
                    }
                    if func.name.node == "as_u32" {
                        self.canonical_as_u32 = false;
                    }
                    // #[intrinsic] is only allowed in vm.*/std.*/os.*/ext.* modules
                    if func.intrinsic.is_some() && !is_std_module {
                        self.error(
                            format!(
                                "#[intrinsic] is only allowed in vm.*/std.*/os.* modules, \
                                 not in '{}'",
                                file.name.node
                            ),
                            func.name.span,
                        );
                    }
                    if func.type_params.is_empty() {
                        // Non-generic function: resolve immediately.
                        let params: Vec<(String, Ty)> = func
                            .params
                            .iter()
                            .map(|p| (p.name.node.clone(), self.resolve_type(&p.ty.node)))
                            .collect();
                        let return_ty = func
                            .return_ty
                            .as_ref()
                            .map(|t| self.resolve_type(&t.node))
                            .unwrap_or(Ty::Unit);
                        self.validate_intrinsic(func, &params, &return_ty);
                        self.generic_fns.remove(&func.name.node);
                        self.functions.insert(
                            func.name.node.clone(),
                            FnSig {
                                params,
                                return_ty,
                                intrinsic: func
                                    .intrinsic
                                    .as_ref()
                                    .map(|i| crate::ast::intrinsic_name(&i.node).to_string()),
                            },
                        );
                    } else {
                        if func.intrinsic.is_some() {
                            self.error(
                                "generic intrinsic declarations have no fixed target ABI".into(),
                                func.name.span,
                            );
                        }
                        // Generic function: store unresolved for monomorphization.
                        let gdef = GenericFnDef {
                            canonical_name: None,
                            structs: BTreeMap::new(),
                            constants: BTreeMap::new(),
                            type_params: func.type_params.iter().map(|p| p.node.clone()).collect(),
                            params: func
                                .params
                                .iter()
                                .map(|p| (p.name.node.clone(), p.ty.node.clone()))
                                .collect(),
                            return_ty: func.return_ty.as_ref().map(|t| t.node.clone()),
                        };
                        self.functions.remove(&func.name.node);
                        self.generic_fns.insert(func.name.node.clone(), gdef);
                    }
                }
                Item::Const(_) => {}
                Item::Event(edef) => {
                    let mut names = BTreeSet::new();
                    for field in &edef.fields {
                        if !names.insert(&field.name.node) {
                            self.error(
                                format!(
                                    "duplicate field '{}' in event '{}'",
                                    field.name.node, edef.name.node
                                ),
                                field.name.span,
                            );
                        }
                    }
                    if edef.fields.len() > 9 {
                        self.error(
                            format!(
                                "event '{}' has {} fields, max is 9",
                                edef.name.node,
                                edef.fields.len()
                            ),
                            edef.name.span,
                        );
                    }
                    let fields: Vec<(String, Ty)> = edef
                        .fields
                        .iter()
                        .map(|f| {
                            let ty = self.resolve_type(&f.ty.node);
                            (f.name.node.clone(), ty)
                        })
                        .collect();
                    let words = fields
                        .iter()
                        .try_fold(0u32, |n, (_, ty)| Some(n.saturating_add(ty.width()?)));
                    if words.is_none() {
                        self.error("Noun has no event payload layout".into(), edef.name.span);
                    }
                    if let Some(words) = words.filter(|w| *w > 9) {
                        self.error(
                            format!(
                                "event '{}' has {} payload words, max is 9",
                                edef.name.node, words
                            ),
                            edef.name.span,
                        );
                    }
                    self.events.insert(edef.name.node.clone(), fields);
                }
            }
        }

        self.check_noun_boundaries(file);
        for item in &file.items {
            if let Item::Fn(f) = &item.node {
                if let Some(generic) = self.generic_fns.get_mut(&f.name.node) {
                    generic.structs = self.structs.clone();
                    generic.constants = self.constants.clone();
                }
            }
        }

        // Recursion detection: build call graph and reject cycles
        self.detect_recursion(file);

        // Second pass: type check function bodies
        for item in &file.items {
            if !self.is_item_cfg_active(&item.node) {
                continue;
            }
            if let Item::Fn(func) = &item.node {
                self.check_fn(func);
            }
        }

        // Unused import detection: collect used module prefixes from all calls
        let mut used_prefixes: BTreeSet<String> = BTreeSet::new();
        for item in &file.items {
            if !self.is_item_cfg_active(&item.node) {
                continue;
            }
            if let Item::Fn(func) = &item.node {
                if let Some(body) = &func.body {
                    Self::collect_used_modules_block(&body.node, &mut used_prefixes);
                }
            }
            if let Item::Const(constant) = &item.node {
                Self::collect_used_modules_expr(&constant.value.node, &mut used_prefixes);
            }
        }
        for use_stmt in &file.uses {
            let module_path = use_stmt.node.as_dotted();
            // Short alias: last segment
            let short = module_path
                .rsplit('.')
                .next()
                .unwrap_or(&module_path)
                .to_string();
            if !used_prefixes.contains(&short) && !used_prefixes.contains(&module_path) {
                self.warning(format!("unused import '{}'", module_path), use_stmt.span);
            }
        }

        let function_requirements = self.infer_requirements(file);
        if file.kind == FileKind::Program {
            if let Some(errors) = capabilities::entry_errors(
                file,
                &function_requirements,
                &self.available_intrinsics,
                &self.cfg_flags,
                &self.target_config.name,
            ) {
                self.diagnostics.extend(errors);
            }
        }

        // Collect exports (pub items only)
        let module_name = file.name.node.clone();
        let mut exported_fns = Vec::new();
        let exported_consts = resolved_constants
            .locals
            .values()
            .filter(|b| b.public)
            .map(|b| (b.name.clone(), b.ty.clone(), b.raw))
            .collect();
        let mut exported_structs = Vec::new();

        for item in &file.items {
            if !self.is_item_cfg_active(&item.node) {
                continue;
            }
            match &item.node {
                Item::Fn(func) if func.is_pub => {
                    if !func.type_params.is_empty() {
                        // An unresolved signature is exported separately; never pretend N=0.
                        continue;
                    }
                    let params: Vec<(String, Ty)> = func
                        .params
                        .iter()
                        .map(|p| (p.name.node.clone(), self.resolve_type(&p.ty.node)))
                        .collect();
                    let return_ty = func
                        .return_ty
                        .as_ref()
                        .map(|t| self.resolve_type(&t.node))
                        .unwrap_or(Ty::Unit);
                    exported_fns.push((func.name.node.clone(), params, return_ty));
                }
                Item::Struct(sdef) if sdef.is_pub => {
                    if let Some(sty) = self.structs.get(&sdef.name.node) {
                        exported_structs.push(sty.clone());
                    }
                }
                _ => {}
            }
        }

        let has_errors = self
            .diagnostics
            .iter()
            .any(|d| d.severity == crate::diagnostic::Severity::Error);
        if has_errors {
            Err(self.diagnostics)
        } else {
            Ok(ModuleExports {
                resolved_constants,
                module_name,
                generic_functions: file
                    .items
                    .iter()
                    .filter_map(|item| match &item.node {
                        Item::Fn(function)
                            if function.is_pub
                                && self.is_item_cfg_active(&item.node)
                                && !function.type_params.is_empty() =>
                        {
                            self.generic_fns
                                .get(&function.name.node)
                                .cloned()
                                .map(|definition| (function.name.node.clone(), definition))
                        }
                        _ => None,
                    })
                    .collect(),
                generic_calls: self.generic_calls,
                direct_intrinsics: exported_fns
                    .iter()
                    .filter_map(|(name, _, _)| {
                        self.functions
                            .get(name)?
                            .intrinsic
                            .clone()
                            .map(|intrinsic| (name.clone(), intrinsic))
                    })
                    .collect(),
                functions: exported_fns,
                function_requirements,
                constants: exported_consts,
                structs: exported_structs,
                warnings: self.diagnostics,
                mono_instances: self.mono_instances,
                call_resolutions: self.call_resolutions,
            })
        }
    }
}
