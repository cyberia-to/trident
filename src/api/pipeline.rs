// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Shared project preparation pipeline.
//!
//! Extracts the resolve → parse → typecheck loop that was duplicated across
//! many public API functions in `lib.rs`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::CompileOptions;
use crate::ast;
use crate::diagnostic::{Diagnostic, render_diagnostics};
use crate::typecheck::ModuleExports;

/// A single parsed module: path, source text, and parsed AST.
pub(crate) struct ParsedModule {
    pub file_path: PathBuf,
    pub source: String,
    pub file: ast::File,
}

/// A fully resolved, parsed, and type-checked project.
pub(crate) struct PreparedProject {
    pub modules: Vec<ParsedModule>,
    pub exports: Vec<ModuleExports>,
}

impl PreparedProject {
    /// The shared nox lowering for assembly, cost and bundle state metadata.
    /// Every path consumes the same resolved modules and conditional flags.
    pub(crate) fn lower_nox(
        &self,
        options: &CompileOptions,
    ) -> Result<(crate::ir::tree::lower::Noun, bool), Vec<Diagnostic>> {
        super::require_nox_target(options)?;
        let entry = self
            .modules
            .iter()
            .find(|m| m.file.kind == ast::FileKind::Program)
            .or_else(|| self.modules.last())
            .ok_or_else(|| {
                vec![Diagnostic::error(
                    "no entry module found".to_string(),
                    crate::span::Span::dummy(),
                )]
            })?;
        let files: Vec<_> = self.modules.iter().map(|m| &m.file).collect();
        let mut compiler = crate::ir::tree::lower::nox::NoxCompiler::new();
        let noun = compiler
            .compile_modules(&files, &entry.file, &options.cfg_flags)
            .map_err(|e| vec![Diagnostic::error(e, crate::span::Span::dummy())])?;
        Ok((noun, compiler.reads_state()))
    }

    /// Build a project from an entry path using the given compile options.
    ///
    /// This performs the resolve → parse → typecheck pipeline that is shared
    /// across `compile_project`, `run_tests`, and the tree-target cost path
    /// (`nox_cost_project`). Stack-target cost analysis moved to the
    /// warrior with the lowering it priced.
    pub fn build(entry_path: &Path, options: &CompileOptions) -> Result<Self, Vec<Diagnostic>> {
        Self::build_inner(entry_path, options, true, false)
    }

    /// Like [`build`], but never renders diagnostics — the caller decides what
    /// to do with the returned errors. Used by best-effort probes (e.g. the
    /// nox reduction column in `trident bench`) that expect many programs to
    /// fall outside the target surface and must not spam the terminal.
    pub fn build_quiet(
        entry_path: &Path,
        options: &CompileOptions,
    ) -> Result<Self, Vec<Diagnostic>> {
        Self::build_inner(entry_path, options, false, false)
    }

    /// Prepare declarations and bodies without choosing the application's entry.
    pub fn build_tests(
        entry_path: &Path,
        options: &CompileOptions,
    ) -> Result<Self, Vec<Diagnostic>> {
        Self::build_inner(entry_path, options, false, true)
    }

    fn build_inner(
        entry_path: &Path,
        options: &CompileOptions,
        render: bool,
        tests: bool,
    ) -> Result<Self, Vec<Diagnostic>> {
        options.validate()?;
        let resolved = crate::resolve::resolve_modules_with_sources(
            entry_path,
            options.dep_dirs.clone(),
            options.library_sources(),
        )?;

        let mut modules = Vec::new();
        for m in &resolved {
            let file = crate::parse_source(&m.source, &m.file_path.to_string_lossy())?;
            modules.push(ParsedModule {
                file_path: m.file_path.clone(),
                source: m.source.clone(),
                file,
            });
        }

        let exports = Self::check_and_specialize(&mut modules, options, render, tests)?;

        if let Some((entry, exports)) = modules
            .iter()
            .zip(&exports)
            .find(|(m, _)| m.file.kind == ast::FileKind::Program)
            .or_else(|| modules.last().zip(exports.last()))
        {
            if !tests {
                exports.check_entry_requirements(&entry.file, options)?;
            }
        }
        Ok(PreparedProject { modules, exports })
    }

    /// Resolve concrete generic calls before either backend sees the project.
    pub(crate) fn check_and_specialize(
        modules: &mut [ParsedModule],
        options: &CompileOptions,
        render: bool,
        tests: bool,
    ) -> Result<Vec<ModuleExports>, Vec<Diagnostic>> {
        use crate::ast::{Item, ModulePath};
        use crate::span::{Span, Spanned};
        type Key = (usize, String, Vec<u64>);
        let mut instances: BTreeMap<Key, String> = BTreeMap::new();
        for _ in 0..128 {
            let mut exports = Vec::new();
            for pm in modules.iter() {
                let mut tc = options.checker();
                for e in &exports {
                    tc.import_module(e);
                }
                let mut view = pm.file.clone();
                if tests {
                    view.kind = ast::FileKind::Module;
                }
                match tc.check_file(&view) {
                    Ok(e) => exports.push(e),
                    Err(errors) => {
                        if render {
                            render_diagnostics(
                                &errors,
                                &pm.file_path.to_string_lossy(),
                                &pm.source,
                            );
                        }
                        return Err(errors);
                    }
                }
            }
            let mut replacements = vec![BTreeMap::new(); modules.len()];
            let mut additions = Vec::new();
            for (caller, export) in exports.iter().enumerate() {
                for (site, instance) in &export.generic_calls {
                    let (owner, base) = if let Some((prefix, base)) = instance.name.rsplit_once('.')
                    {
                        let owner = modules
                            .iter()
                            .position(|m| m.file.name.node == prefix)
                            .ok_or_else(|| {
                                vec![Diagnostic::error(
                                    "generic module not resolved".into(),
                                    Span::dummy(),
                                )]
                            })?;
                        (owner, base.to_string())
                    } else {
                        (caller, instance.name.clone())
                    };
                    let key = (owner, base.clone(), instance.size_args.clone());
                    let name = if let Some(name) = instances.get(&key) {
                        name.clone()
                    } else {
                        if instances.len() >= 1024 {
                            return Err(vec![Diagnostic::error(
                                "generic instance limit exceeded".into(),
                                Span::dummy(),
                            )]);
                        }
                        let definition = modules[owner]
                            .file
                            .items
                            .iter()
                            .find_map(|item| match &item.node {
                                Item::Fn(f)
                                    if f.name.node == base
                                        && !f.type_params.is_empty()
                                        && f.cfg.as_ref().is_none_or(|cfg| {
                                            options.cfg_flags.contains(&cfg.node)
                                        }) =>
                                {
                                    Some(f.clone())
                                }
                                _ => None,
                            })
                            .ok_or_else(|| {
                                vec![Diagnostic::error(
                                    "generic definition not resolved".into(),
                                    Span::dummy(),
                                )]
                            })?;
                        let mut serial = instances.len();
                        let name = loop {
                            let candidate = format!("trident_mono_{serial}");
                            let occupied = modules[owner].file.items.iter().any(|item| matches!(&item.node, Item::Fn(f) if f.name.node == candidate))
                                || instances.iter().any(|((module,_,_), name)| *module == owner && name == &candidate);
                            if !occupied {
                                break candidate;
                            }
                            serial += 1;
                        };
                        let mut constants = BTreeMap::new();
                        for export in &exports[..owner] {
                            for (constant, _, value) in &export.constants {
                                constants
                                    .insert(format!("{}.{}", export.module_name, constant), *value);
                                constants.insert(
                                    format!(
                                        "{}.{}",
                                        export.module_name.rsplit('.').next().unwrap(),
                                        constant
                                    ),
                                    *value,
                                );
                            }
                        }
                        for item in &modules[owner].file.items {
                            if let Item::Const(c) = &item.node {
                                if c.cfg
                                    .as_ref()
                                    .is_none_or(|cfg| options.cfg_flags.contains(&cfg.node))
                                {
                                    if let ast::Expr::Literal(ast::Literal::Integer(value)) =
                                        c.value.node
                                    {
                                        constants.insert(c.name.node.clone(), value);
                                    }
                                }
                            }
                        }
                        let function = crate::typecheck::specialize::concrete_function(
                            &definition,
                            &instance.size_args,
                            name.clone(),
                            &constants,
                        )
                        .map_err(|e| vec![Diagnostic::error(e, definition.name.span)])?;
                        additions.push((
                            owner,
                            Spanned::new(Item::Fn(function), definition.name.span),
                        ));
                        instances.insert(key, name.clone());
                        name
                    };
                    let path = if owner == caller {
                        ModulePath(vec![name])
                    } else {
                        let mut path: Vec<_> = modules[owner]
                            .file
                            .name
                            .node
                            .split('.')
                            .map(str::to_string)
                            .collect();
                        path.push(name);
                        ModulePath(path)
                    };
                    replacements[caller].insert(site.clone(), path);
                }
            }
            if replacements.iter().all(BTreeMap::is_empty) {
                if render {
                    for (pm, e) in modules.iter().zip(&exports) {
                        if !e.warnings.is_empty() {
                            render_diagnostics(
                                &e.warnings,
                                &pm.file_path.to_string_lossy(),
                                &pm.source,
                            );
                        }
                    }
                }
                return Ok(exports);
            }
            for (pm, replacements) in modules.iter_mut().zip(&replacements) {
                crate::typecheck::specialize::rewrite_calls(&mut pm.file, replacements)
                    .map_err(|e| vec![Diagnostic::error(e, Span::dummy())])?;
            }
            for (owner, function) in additions {
                modules[owner].file.items.push(function);
            }
        }
        Err(vec![Diagnostic::error(
            "generic instantiation depth exceeded".into(),
            Span::dummy(),
        )])
    }

    pub(crate) fn source(
        file: ast::File,
        source: &str,
        filename: &str,
        options: &CompileOptions,
    ) -> Result<(ast::File, ModuleExports), Vec<Diagnostic>> {
        let mut modules = vec![ParsedModule {
            file_path: PathBuf::from(filename),
            source: source.into(),
            file,
        }];
        let mut exports = Self::check_and_specialize(&mut modules, options, false, false)?;
        Ok((modules.remove(0).file, exports.remove(0)))
    }

    /// Build a global intrinsic map from all modules.
    ///
    /// Maps function names (short, qualified, and short-alias qualified) to
    /// their `#[intrinsic(...)]` values.
    pub fn intrinsic_map(&self) -> BTreeMap<String, String> {
        let mut map = BTreeMap::new();
        for pm in &self.modules {
            for item in &pm.file.items {
                if let ast::Item::Fn(func) = &item.node {
                    if let Some(ref intrinsic) = func.intrinsic {
                        let intr_value = if let Some(start) = intrinsic.node.find('(') {
                            let end = intrinsic.node.rfind(')').unwrap_or(intrinsic.node.len());
                            intrinsic.node[start + 1..end].to_string()
                        } else {
                            intrinsic.node.clone()
                        };
                        // Short function name
                        map.insert(func.name.node.clone(), intr_value.clone());
                        // Qualified name (module.func)
                        let qualified = format!("{}.{}", pm.file.name.node, func.name.node);
                        map.insert(qualified, intr_value.clone());
                        // Short alias (hash.func for std.hash)
                        if let Some(short) = pm.file.name.node.rsplit('.').next() {
                            if short != pm.file.name.node {
                                let short_qualified = format!("{}.{}", short, func.name.node);
                                map.insert(short_qualified, intr_value.clone());
                            }
                        }
                    }
                }
            }
        }
        map
    }

    /// Build module alias map: short name -> full name for dotted modules.
    pub fn module_aliases(&self) -> BTreeMap<String, String> {
        let mut aliases = BTreeMap::new();
        for pm in &self.modules {
            let full_name = &pm.file.name.node;
            if let Some(short) = full_name.rsplit('.').next() {
                if short != full_name.as_str() {
                    aliases.insert(short.to_string(), full_name.clone());
                }
            }
        }
        aliases
    }

    /// Build external constants map from all module exports.
    pub fn external_constants(&self) -> BTreeMap<String, u64> {
        let mut constants = BTreeMap::new();
        for exp in &self.exports {
            let full = &exp.module_name;
            let short = full.rsplit('.').next().unwrap_or(full);
            let has_short = short != full;
            for (const_name, _ty, value) in &exp.constants {
                let qualified = format!("{}.{}", full, const_name);
                constants.insert(qualified, *value);
                if has_short {
                    let short_qualified = format!("{}.{}", short, const_name);
                    constants.insert(short_qualified, *value);
                }
            }
        }
        constants
    }
}
