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

use crate::ast;
use crate::diagnostic::{render_diagnostics, Diagnostic};
use crate::typecheck::ModuleExports;
use crate::CompileOptions;

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
    pub native_origins: BTreeMap<String, (String, Vec<u64>)>,
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

        let mut native_origins = BTreeMap::new();
        let exports = Self::specialize_with_origins(
            &mut modules,
            options,
            render,
            tests,
            &mut native_origins,
        )?;

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
        Ok(PreparedProject {
            modules,
            exports,
            native_origins,
        })
    }

    /// Resolve concrete generic calls before either backend sees the project.
    pub(crate) fn check_and_specialize(
        modules: &mut [ParsedModule],
        options: &CompileOptions,
        render: bool,
        tests: bool,
    ) -> Result<Vec<ModuleExports>, Vec<Diagnostic>> {
        Self::specialize_with_origins(modules, options, render, tests, &mut BTreeMap::new())
    }

    fn specialize_with_origins(
        modules: &mut [ParsedModule],
        options: &CompileOptions,
        render: bool,
        tests: bool,
        origins: &mut BTreeMap<String, (String, Vec<u64>)>,
    ) -> Result<Vec<ModuleExports>, Vec<Diagnostic>> {
        use crate::ast::{Item, ModulePath};
        use crate::span::{Span, Spanned};
        // Logical ownership must be unique before imports or generic copies can
        // confer access. Resolver keys may be legacy aliases or scanned headers;
        // only parsed module names establish the semantic identity.
        let mut owners = BTreeMap::new();
        for module in modules.iter() {
            if let Some(previous) = owners.insert(&module.file.name.node, &module.file_path) {
                return Err(vec![Diagnostic::error(
                    format!(
                        "duplicate module '{}' declared by '{}' and '{}'",
                        module.file.name.node,
                        previous.display(),
                        module.file_path.display()
                    ),
                    module.file.name.span,
                )]);
            }
        }
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
                        origins.insert(
                            format!("{}.{}", modules[owner].file.name.node, name),
                            (
                                format!("{}.{}", modules[owner].file.name.node, base),
                                instance.size_args.clone(),
                            ),
                        );
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
        let (file, exports, _) = Self::source_with_origins(file, source, filename, options)?;
        Ok((file, exports))
    }

    pub(crate) fn source_with_origins(
        file: ast::File,
        source: &str,
        filename: &str,
        options: &CompileOptions,
    ) -> Result<
        (
            ast::File,
            ModuleExports,
            BTreeMap<String, (String, Vec<u64>)>,
        ),
        Vec<Diagnostic>,
    > {
        let mut modules = vec![ParsedModule {
            file_path: PathBuf::from(filename),
            source: source.into(),
            file,
        }];
        let mut origins = BTreeMap::new();
        let mut exports =
            Self::specialize_with_origins(&mut modules, options, false, false, &mut origins)?;
        Ok((modules.remove(0).file, exports.remove(0), origins))
    }

    /// Import the same active, public intrinsic identities as the typechecker.
    /// Bare names belong to each module's own declaration scan.
    pub fn intrinsic_map(&self, before: usize) -> BTreeMap<String, String> {
        let mut map = BTreeMap::new();
        let direct: BTreeMap<_, _> = self
            .exports
            .iter()
            .take(before)
            .flat_map(|exports| {
                exports.direct_intrinsics.iter().map(|(name, intrinsic)| {
                    (
                        format!("{}.{}", exports.module_name, name),
                        intrinsic.clone(),
                    )
                })
            })
            .collect();
        for (visible, canonical) in self.function_aliases(before) {
            if let Some(intrinsic) = direct.get(&canonical) {
                map.insert(visible, intrinsic.clone());
            }
        }
        map
    }

    /// Callable bindings mirror TypeChecker::import_module, per owner scope.
    pub fn function_aliases(&self, before: usize) -> BTreeMap<String, String> {
        let mut aliases = BTreeMap::new();
        for exports in self.exports.iter().take(before) {
            let full = &exports.module_name;
            let short = full.rsplit('.').next().unwrap_or(full);
            for name in exports
                .functions
                .iter()
                .map(|(name, _, _)| name)
                .chain(exports.generic_functions.iter().map(|(name, _)| name))
            {
                let canonical = format!("{full}.{name}");
                aliases.insert(canonical.clone(), canonical.clone());
                if short != full {
                    aliases.insert(format!("{short}.{name}"), canonical);
                }
            }
        }
        aliases
    }

    /// Build module alias map: short name -> full name for dotted modules.
    pub fn module_aliases(&self, before: usize) -> BTreeMap<String, String> {
        let mut aliases = BTreeMap::new();
        for pm in self.modules.iter().take(before) {
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
    pub fn external_constants(&self, before: usize) -> BTreeMap<String, u64> {
        let mut constants = BTreeMap::new();
        for exp in self.exports.iter().take(before) {
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
