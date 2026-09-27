use super::{nouns::data, support};
use nox::{artifact, Reduction};
use support::Compilation;

fn compile(entry: &str, dependencies: &[(&str, &str)], profile: u64) -> Compilation {
    let mut modules = vec![support::module(entry.as_bytes())];
    for (owner, source) in dependencies {
        let mut module = support::module(source.as_bytes());
        module.path = (*owner).into();
        modules.push(module);
    }
    modules.sort_by(|a, b| a.path.cmp(&b.path));
    let mut options = support::options();
    options.input = profile;
    options.output = profile;
    support::try_compile_only_package(&modules, "sample", "main", options, data::caps()).unwrap()
}

fn program(result: Compilation) -> Vec<u8> {
    match result {
        Compilation::Program { bytes, .. } => bytes,
        other => panic!("{other:?}"),
    }
}

fn fields(bytes: &[u8]) -> (u64, u64, Vec<u8>) {
    let mut arena = Reduction::<{ 1 << 18 }>::try_new_boxed().unwrap();
    let root = artifact::decode(&mut arena, bytes, trident::NATIVE_ARTIFACT_LIMITS).unwrap();
    assert_eq!(
        support::data::value(&arena, arena.head(root).unwrap()).unwrap(),
        0x41525431
    );
    let mut body = arena.tail(root).unwrap();
    assert_eq!(
        support::data::value(&arena, arena.head(body).unwrap()).unwrap(),
        0
    );
    body = arena.tail(body).unwrap();
    let input = support::data::value(&arena, arena.head(body).unwrap()).unwrap();
    body = arena.tail(body).unwrap();
    let output = support::data::value(&arena, arena.head(body).unwrap()).unwrap();
    body = arena.tail(body).unwrap();
    let formula = arena.head(body).unwrap();
    assert_eq!(
        support::data::value(&arena, arena.tail(body).unwrap()).unwrap(),
        0
    );
    (
        input,
        output,
        artifact::encode(&arena, formula, trident::NATIVE_ARTIFACT_LIMITS).unwrap(),
    )
}

#[test]
fn generated_profile_is_explicit_and_preserves_the_complete_formula() {
    support::worker(|| {
        for source in [
            "program sample fn main(input:Noun)->Noun{input}",
            "program sample fn helper(x:Noun)->Noun{nox_noun_pair(x,nox_noun_atom(7))} fn main(input:Noun)->Noun{helper(input)}",
            "program sample fn main()->Field{7} fn main(input:Noun)->Noun{input}",
        ] {
            let raw = program(compile(source, &[], 0));
            let compiler = program(compile(source, &[], 1));
            let (raw_in, raw_out, raw_formula) = fields(&raw);
            let (input, output, formula) = fields(&compiler);
            assert_eq!((raw_in, raw_out), (0, 0));
            assert_eq!((input, output), (1, 1));
            assert_eq!(formula, raw_formula);
            assert_ne!(raw, compiler);
            assert_eq!(data::run(&raw, &data::nested()).unwrap().bytes,
                       data::run(&compiler, &data::nested()).unwrap().bytes);
        }
        assert_eq!(
            fields(&program(compile(
                "program sample fn main()->Field{7}",
                &[],
                0
            )))
            .0,
            0
        );
    });
}

#[test]
fn compiler_profile_requires_final_structured_entry_before_body_checking() {
    support::worker(|| {
        for body in [
            "fn main()->Field{7}",
            "fn main()->Field{missing}",
            "fn main(x:Noun)->Field{7}",
            "fn main(x:Field)->Noun{nox_noun_atom(x)}",
            "fn main(input:Noun)->Noun{input} fn main()->Field{7}",
        ] {
            for imports in [false, true] {
                let entry = format!(
                    "program sample {} {body}",
                    if imports { "use dep" } else { "" }
                );
                let deps = [("dep", "module dep pub fn value()->Field{9}")];
                match compile(&entry, if imports { &deps } else { &[] }, 1) {
                    Compilation::Errors(errors) => {
                        assert_eq!(errors.len(), 1);
                        let error = &errors[0];
                        assert_eq!((error.module, error.code), (u32::from(imports), 3));
                        let start = entry.rfind("main(").unwrap();
                        assert_eq!(
                            (error.start as usize, error.end as usize),
                            (start, start + 4)
                        );
                    }
                    other => panic!("{entry}: {other:?}"),
                }
            }
        }
    });
}

#[test]
fn imported_scalar_helpers_do_not_change_generated_structured_entry_profiles() {
    support::worker(|| {
        let entry = "program sample use dep fn main(input:Noun)->Noun{nox_noun_pair(input,nox_noun_atom(dep.value()))}";
        let deps = [("dep", "module dep pub fn value()->Field{9}")];
        let raw = program(compile(entry, &deps, 0));
        let compiler = program(compile(entry, &deps, 1));
        assert_eq!(fields(&raw).2, fields(&compiler).2);
        assert_eq!((fields(&compiler).0, fields(&compiler).1), (1, 1));
        let input = data::nested();
        let expected = data::Noun::pair(input.clone(), data::Noun::Atom(9)).encoded();
        assert_eq!(data::run(&raw, &input).unwrap().bytes, expected);
        assert_eq!(data::run(&compiler, &input).unwrap().bytes, expected);
    });
}
