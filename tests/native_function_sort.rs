//! Real declaration shapes exercise only the native ordering component.
#![allow(dead_code)]
#[path = "../examples/selfhost_data/model.rs"]
mod data;
#[path = "../examples/selfhost_jobs/schema.rs"]
mod schema;
#[path = "native_control/support.rs"]
mod support;

use nox::{artifact, sequential, NoTrace, Outcome, Reduction};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use trident::{ast::Item, NativeArtifactProfile, NATIVE_ARTIFACT_LIMITS as LIMITS};

struct Module {
    name: String,
    source: String,
    functions: Vec<(String, u32, u32)>,
    imports: Vec<String>,
}

fn module(name: &str, source: String) -> Module {
    let parsed = trident::parse_source_silent(&source, name).unwrap();
    let functions = parsed
        .items
        .iter()
        .filter_map(|item| match &item.node {
            Item::Fn(f) => Some((f.name.node.clone(), f.name.span.start, f.name.span.end)),
            _ => None,
        })
        .collect();
    Module {
        name: name.into(),
        source,
        functions,
        imports: parsed.uses.iter().map(|u| u.node.as_dotted()).collect(),
    }
}

fn closure(root: &Path) -> Vec<Module> {
    fn load(root: &Path, name: &str, modules: &mut BTreeMap<String, Module>) {
        if modules.contains_key(name) {
            return;
        }
        let path = if name == "native_compiler" {
            PathBuf::from("compiler/nox/main.tri")
        } else {
            PathBuf::from("lib").join(name.replace('.', "/") + ".tri")
        };
        let item = module(name, std::fs::read_to_string(root.join(path)).unwrap());
        let imports = item.imports.clone();
        modules.insert(name.into(), item);
        for import in imports {
            load(root, &import, modules);
        }
    }
    fn visit(
        name: &str,
        modules: &BTreeMap<String, Module>,
        seen: &mut BTreeSet<String>,
        order: &mut Vec<String>,
    ) {
        if !seen.insert(name.into()) {
            return;
        }
        for import in &modules[name].imports {
            visit(import, modules, seen, order);
        }
        order.push(name.into());
    }
    let mut modules = BTreeMap::new();
    load(root, "native_compiler", &mut modules);
    let mut order = Vec::new();
    let mut seen = BTreeSet::new();
    for name in modules.keys() {
        visit(name, &modules, &mut seen, &mut order);
    }
    order
        .into_iter()
        .map(|name| modules.remove(&name).unwrap())
        .collect()
}

// Keep every original function and owner name and their declaration order.
// Ordering consumes name spans only; compact sources isolate that stage from
// full source-byte admission, whose limits are a separate compiler-scale gate.
fn compact_names(modules: &mut [Module]) {
    for module in modules {
        module.source.clear();
        for (name, start, end) in &mut module.functions {
            *start = module.source.len() as u32;
            module.source.push_str(name);
            *end = module.source.len() as u32;
            module.source.push('\n');
        }
    }
}

fn component(ranked: bool) -> Vec<u8> {
    if let Some(path) = std::env::var_os("TRIDENT_SORT_COMPONENT") {
        return std::fs::read(path).unwrap();
    }
    let mut options = trident::CompileOptions::default();
    if let Some(path) = std::env::var_os("TRIDENT_SORT_REFERENCE_SOURCE") {
        options.module_sources.insert(
            "std.compiler.nox.module_function_order".into(),
            std::fs::read_to_string(path).unwrap(),
        );
    }
    let fixture = if ranked {
        "tests/fixtures/native_module_function_ranked_sort.tri"
    } else {
        "tests/fixtures/native_module_function_sort.tri"
    };
    let code = trident::compile_native_artifact_project(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join(fixture),
        &options,
        NativeArtifactProfile::RawNoun,
        LIMITS,
    )
    .unwrap()
    .bytes;
    if let Some(path) = std::env::var_os("TRIDENT_SORT_SAVE_COMPONENT") {
        std::fs::write(path, &code).unwrap();
    }
    code
}

fn check<const N: usize>(
    code: &[u8],
    ranked: bool,
    label: &str,
    modules: &[Module],
    roots: &[u64],
    cap: u32,
) {
    let budget = std::env::var("TRIDENT_SORT_BUDGET")
        .map(|value| value.parse::<u64>().unwrap())
        .unwrap_or(1_000_000_000);
    let mut arena = Reduction::<N>::try_new_boxed().unwrap();
    assert!(arena.limit_allocations((N / 4 * 3) as u32));
    let artifact = artifact::decode(&mut arena, code, LIMITS).unwrap();
    let mut cursor = arena.tail(artifact).unwrap();
    for _ in 0..3 {
        cursor = arena.tail(cursor).unwrap();
    }
    let formula = arena.head(cursor).unwrap();
    let mut functions = Vec::new();
    let mut owners = Vec::new();
    let mut source_pairs = Vec::new();
    let mut keys = Vec::new();
    let zero = schema::atom(&mut arena, 0).unwrap();
    for (owner, module) in modules.iter().enumerate() {
        let name = schema::bytes(&mut arena, module.name.as_bytes()).unwrap();
        let source = schema::bytes(&mut arena, module.source.as_bytes()).unwrap();
        source_pairs.push(schema::pair(&mut arena, name, source).unwrap());
        for (name, start, end) in &module.functions {
            let start = schema::atom(&mut arena, (*start).into()).unwrap();
            let end = schema::atom(&mut arena, (*end).into()).unwrap();
            let span = schema::pair(&mut arena, start, end).unwrap();
            let empty = schema::pair(&mut arena, zero, zero).unwrap();
            let returns_body = schema::pair(&mut arena, zero, empty).unwrap();
            let metadata = schema::pair(&mut arena, empty, returns_body).unwrap();
            functions.push(schema::pair(&mut arena, span, metadata).unwrap());
            owners.push(schema::atom(&mut arena, owner as u64).unwrap());
            keys.push((module.name.as_str(), name.as_str()));
        }
    }
    let functions = schema::seq(&mut arena, &functions).unwrap();
    let owners = schema::seq(&mut arena, &owners).unwrap();
    let modules_noun = schema::seq(&mut arena, &source_pairs).unwrap();
    let root_values: Vec<_> = roots
        .iter()
        .map(|id| schema::atom(&mut arena, *id).unwrap())
        .collect();
    let root_noun = schema::seq(&mut arena, &root_values).unwrap();
    let mut cap_noun = schema::atom(&mut arena, cap.into()).unwrap();
    if ranked {
        let mut lexical: Vec<_> = modules.iter().map(|m| m.name.as_str()).collect();
        lexical.sort_unstable();
        lexical.dedup();
        assert_eq!(
            lexical.len(),
            modules.len(),
            "JOB1 requires unique module paths"
        );
        let ranks: Vec<_> = modules
            .iter()
            .map(|m| {
                let mut rank = lexical.binary_search(&m.name.as_str()).unwrap() as u64;
                // Reached owners can have large gaps in the admitted package.
                if modules.len() <= 3 && rank as usize + 1 == modules.len() {
                    rank = 65535;
                }
                schema::atom(&mut arena, rank).unwrap()
            })
            .collect();
        let ranks = schema::seq(&mut arena, &ranks).unwrap();
        cap_noun = schema::pair(&mut arena, cap_noun, ranks).unwrap();
    }
    let rest = schema::pair(&mut arena, root_noun, cap_noun).unwrap();
    let rest = schema::pair(&mut arena, modules_noun, rest).unwrap();
    let rest = schema::pair(&mut arena, owners, rest).unwrap();
    let input = schema::pair(&mut arena, functions, rest).unwrap();
    let run = sequential::reduce(
        &mut arena,
        input,
        formula,
        budget,
        sequential::Limits { max_frames: 65536 },
        &mut NoTrace,
    )
    .unwrap();
    let (result, left) = match run.outcome {
        Outcome::Ok(result, left) => (result, left),
        other => panic!(
            "{label}: {other:?}; nodes={}; frames={}",
            arena.count(),
            run.peak_frames
        ),
    };
    let error = data::value(&arena, arena.head(result).unwrap()).unwrap();
    let rest = arena.tail(result).unwrap();
    let next = data::value(&arena, arena.head(rest).unwrap()).unwrap();
    let ids = arena.tail(rest).unwrap();
    let ids = data::Seq::decode(&mut arena, ids, 4096, 1000000).unwrap();
    let actual: Vec<_> = (0..ids.len())
        .map(|i| data::value(&arena, ids.get(&arena, i).unwrap()).unwrap())
        .collect();
    let mut expected: Vec<_> = roots.iter().take(cap as usize).copied().collect();
    expected.sort_by_key(|id| keys[*id as usize]);
    assert_eq!(actual, expected, "{label}");
    assert_eq!(
        error,
        if roots.len() > cap as usize { 7 } else { 0 },
        "{label}"
    );
    assert_eq!(next as usize, roots.len().min(cap as usize), "{label}");
    eprintln!("sort-component {label}: ranked={ranked} modules={} functions={} source_bytes={} reductions={} nodes={} frames={}",
        modules.len(), keys.len(), modules.iter().map(|m| m.source.len()).sum::<usize>(),
        budget-left, arena.count(), run.peak_frames);
}

fn semantics(ranked: bool) {
    support::worker(move || {
        let code = component(ranked);
        let modules = vec![
            module("a.b", "module a.b fn a(){} fn z(){}".into()),
            module("a", "module a fn z(){} fn a(){} fn a(){}".into()),
            module("a_b", "module a_b fn a(){}".into()),
        ];
        for (label, roots, cap) in [
            ("empty", vec![], 0),
            ("one", vec![3], 1),
            ("pairs-stable", vec![5, 0, 4, 3, 2, 1, 0], 7),
            ("zero-cap", vec![5], 0),
            ("prefix-cap", vec![5, 0, 4, 3, 2, 1, 0], 4),
        ] {
            check::<{ 1 << 18 }>(&code, ranked, label, &modules, &roots, cap);
        }
        let mut source = "module aligned ".to_string();
        for lane in 0..4 {
            for name in [
                "a",
                "aaaa",
                "aaaaa",
                "aaaab",
                "common_prefix_a",
                "common_prefix_ab",
            ] {
                while (source.len() + 3) % 4 != lane {
                    source.push(' ');
                }
                source.push_str(&format!("fn {name}(){{}} "));
            }
        }
        let modules = [module("aligned", source)];
        let roots: Vec<_> = (0..24).rev().collect();
        check::<{ 1 << 20 }>(&code, ranked, "all-name-alignments", &modules, &roots, 24);
    });
}

#[test]
fn native_module_sort_preserves_pairs_stability_and_capacity() {
    semantics(false);
}

#[test]
fn admitted_owner_ranks_preserve_prefix_names_and_stable_members() {
    semantics(true);
}

#[test]
#[ignore = "compiler-scale ordering component: run explicitly in release mode"]
fn native_module_sort_handles_real_compiler_declaration_names() {
    support::worker(|| {
        let ranked = std::env::var_os("TRIDENT_SORT_RANKED").is_some();
        let code = component(ranked);
        let root = std::env::var_os("TRIDENT_SORT_SOURCE_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        let mut modules = closure(&root);
        let exact = std::env::var_os("TRIDENT_SORT_EXACT_SOURCES").is_some();
        if !exact {
            compact_names(&mut modules);
        }
        let count = modules.iter().map(|m| m.functions.len()).sum::<usize>();
        let roots = (0..count as u64).collect::<Vec<_>>();
        let label = if exact {
            "exact-source-declarations"
        } else {
            "compact-name-declarations"
        };
        if std::env::var_os("TRIDENT_SORT_SMALL_ARENA").is_some() {
            check::<{ 1 << 22 }>(&code, ranked, label, &modules, &roots, 4096);
        } else {
            check::<{ 1 << 24 }>(&code, ranked, label, &modules, &roots, 4096);
        }
    });
}
