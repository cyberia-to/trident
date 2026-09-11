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
        Self::build_inner(entry_path, options, true)
    }

    /// Like [`build`], but never renders diagnostics — the caller decides what
    /// to do with the returned errors. Used by best-effort probes (e.g. the
    /// nox reduction column in `trident bench`) that expect many programs to
    /// fall outside the target surface and must not spam the terminal.
    pub fn build_quiet(
        entry_path: &Path,
        options: &CompileOptions,
    ) -> Result<Self, Vec<Diagnostic>> {
        Self::build_inner(entry_path, options, false)
    }

    fn build_inner(
        entry_path: &Path,
        options: &CompileOptions,
        render: bool,
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

        let mut exports: Vec<ModuleExports> = Vec::new();
        for pm in &modules {
            let mut tc = options.checker();
            for e in &exports {
                tc.import_module(e);
            }
            match tc.check_file(&pm.file) {
                Ok(e) => {
                    if render && !e.warnings.is_empty() {
                        render_diagnostics(
                            &e.warnings,
                            &pm.file_path.to_string_lossy(),
                            &pm.source,
                        );
                    }
                    exports.push(e);
                }
                Err(errors) => {
                    if render {
                        render_diagnostics(&errors, &pm.file_path.to_string_lossy(), &pm.source);
                    }
                    return Err(errors);
                }
            }
        }

        if let Some((entry, exports)) = modules
            .iter()
            .zip(&exports)
            .find(|(m, _)| m.file.kind == ast::FileKind::Program)
            .or_else(|| modules.last().zip(exports.last()))
        {
            exports.check_entry_requirements(&entry.file, options)?;
        }
        Ok(PreparedProject { modules, exports })
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
