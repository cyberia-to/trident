use super::*;
/// Compile a multi-module project to a `ProgramBundle` artifact.
///
/// This is the primary entry point for warriors: it produces a
/// self-contained bundle with compiled assembly, cost analysis,
/// function signatures, and metadata.
pub fn compile_to_bundle(
    entry_path: &Path,
    options: &CompileOptions,
) -> Result<crate::runtime::ProgramBundle, Vec<Diagnostic>> {
    use crate::runtime::artifact::BundleCost;

    let tasm = compile_project_with_options(entry_path, options)?;

    // Cost: nox prices in reductions (bill.max, an upper bound — see
    // cost::nox::NoxCost); stack targets never reach this line, since
    // compile_project_with_options above already errored for them
    // (the core stops at TIR — reference/warrior-api.md). A warrior
    // that owns its own bundle assembly fills in its own cost model.
    let bundle_cost = match nox_cost_project(entry_path, options) {
        Ok(nc) => BundleCost {
            table_values: vec![nc.bill.max],
            table_names: vec!["reductions".to_string()],
            padded_height: nc.nodes,
            estimated_proving_ns: 0,
        },
        Err(_) => BundleCost {
            table_values: Vec::new(),
            table_names: Vec::new(),
            padded_height: 0,
            estimated_proving_ns: 0,
        },
    };

    bundle_with_assembly(entry_path, options, tasm, bundle_cost)
}

/// Attach source metadata to assembly and costs produced by a warrior.
/// The warrior owns instruction selection; the core owns source identities.
pub fn bundle_with_assembly(
    entry_path: &Path,
    options: &CompileOptions,
    assembly: String,
    cost: crate::runtime::artifact::BundleCost,
) -> Result<crate::runtime::ProgramBundle, Vec<Diagnostic>> {
    use crate::runtime::artifact::{BundleFunction, ProgramBundle};
    use pipeline::PreparedProject;

    // Parse entry file for function signatures + content hashes.
    let project = PreparedProject::build(entry_path, options)?;
    let entry_file = project
        .modules
        .iter()
        .find(|m| m.file.kind == FileKind::Program)
        .or_else(|| project.modules.last());

    let (functions, entry_point, source_hash) = if let Some(pm) = entry_file {
        let fn_hashes = crate::hash::hash_file(&pm.file);
        let fns: Vec<BundleFunction> = pm
            .file
            .items
            .iter()
            .filter_map(|item| {
                if let ast::Item::Fn(func) = &item.node {
                    if !func.is_test
                        && func
                            .cfg
                            .as_ref()
                            .is_none_or(|flag| options.cfg_flags.contains(&flag.node))
                    {
                        let hash = fn_hashes
                            .get(&func.name.node)
                            .map(|h| h.to_hex())
                            .unwrap_or_default();
                        return Some(BundleFunction {
                            name: func.name.node.clone(),
                            hash,
                            signature: crate::deploy::format_fn_signature(func),
                        });
                    }
                }
                None
            })
            .collect();
        let ep = if fns.iter().any(|f| f.name == "main") {
            "main".to_string()
        } else {
            fns.first()
                .map(|f| f.name.clone())
                .unwrap_or_else(|| "main".to_string())
        };
        let sh = crate::hash::hash_file_content(&pm.file).to_hex();
        (fns, ep, sh)
    } else {
        (Vec::new(), "main".to_string(), String::new())
    };

    let name = entry_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("program")
        .to_string();

    // Tree targets: ask the lowering (the source of truth) whether the
    // program reads persistent state, so the bundle can declare it and the
    // runner knows to cons the BBG root onto the subject.
    let reads_state = if options.target_config.architecture == crate::target::Arch::Tree {
        project.lower_nox(options)?.1
    } else {
        false
    };

    Ok(ProgramBundle {
        name,
        version: "0.1.0".to_string(),
        target_vm: options.target_config.name.clone(),
        target_os: None,
        assembly,
        entry_point,
        functions,
        cost,
        source_hash,
        reads_state,
    })
}
