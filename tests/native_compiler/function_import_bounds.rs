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
fn imported_parameter_allowance_is_shared_and_rejects_before_first_excess_parameter() {
    support::worker(|| {
        boundary(
            &[
                ("a", "module a pub fn f(a:Field,b:Field,c:Field)->Field{a}"),
                ("b", "module b pub fn g(d:Field,e:Field)->Field{d}"),
                ("sample", "program sample use a use b fn main()->Field{14}"),
            ],
            "b",
            "e",
        );
    });
}

#[test]
fn imported_function_allowance_counts_private_and_replaced_declarations_together() {
    support::worker(|| {
        boundary(
            &[
                ("a", "module a pub fn f()->Field{1} fn f()->Field{2}"),
                ("b", "module b fn g()->Field{3} pub fn h()->Field{4}"),
                ("sample", "program sample use a use b fn main()->Field{14}"),
            ],
            "sample",
            "fn",
        );
    });
}
