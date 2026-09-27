use super::{nouns::data, support};
use support::{schema, Compilation};

fn modules(sources: &[(&str, &str)]) -> Vec<schema::Module> {
    let mut modules: Vec<_> = sources
        .iter()
        .map(|(name, source)| {
            let mut module = support::module(source.as_bytes());
            module.path = (*name).into();
            module
        })
        .collect();
    modules.sort_by(|a, b| a.path.cmp(&b.path));
    modules
}

fn boundary(sources: &[(&str, &str)], owner: &str, span: &str) {
    let package = modules(sources);
    let mut caps = data::caps();
    caps[3] = 5;
    let compile = |caps| {
        support::try_compile_only_package(&package, "sample", "main", support::options(), caps)
            .unwrap()
    };
    match compile(caps) {
        Compilation::Program { bytes, .. } => {
            assert_eq!(support::run_artifact(&bytes), 14);
            assert_eq!(bytes, data::compile("program sample fn main()->Field{14}"));
        }
        other => panic!("exact global allowance: {other:?}"),
    }
    caps[3] = 4;
    match compile(caps) {
        Compilation::Errors(errors) => {
            assert_eq!(errors.len(), 1);
            let error = &errors[0];
            assert_eq!(error.code, 7);
            let module = &package[error.module as usize];
            assert_eq!(module.path, owner);
            assert_eq!(
                &module.source[error.start as usize..error.end as usize],
                span.as_bytes()
            );
        }
        other => panic!("one below global allowance: {other:?}"),
    }
}

#[test]
fn nominal_allowance_counts_private_and_repeated_declarations_across_owners() {
    support::worker(|| {
        boundary(
            &[
                ("a", "module a pub struct S{} struct S{}"),
                ("b", "module b struct T{} pub struct U{}"),
                (
                    "sample",
                    "program sample use a use b struct Last{} fn main()->Field{14}",
                ),
            ],
            "sample",
            "Last",
        );
    });
}

#[test]
fn final_type_exports_fit_the_exact_shared_type_declaration_allowance() {
    support::worker(|| {
        boundary(
            &[
                ("a", "module a pub struct A{} pub struct B{}"),
                ("b", "module b pub struct C{} pub struct D{}"),
                (
                    "sample",
                    "program sample use a use b pub struct Last{} fn main()->Field{14}",
                ),
            ],
            "sample",
            "Last",
        );
    });
}
