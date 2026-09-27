use super::*;

fn digest_words(input: &Noun) -> Noun {
    let bytes = input.encoded();
    let word = |i: usize| {
        Atom(u64::from_le_bytes(
            bytes[8 + i * 8..16 + i * 8].try_into().unwrap(),
        ))
    };
    Noun::pair(Noun::pair(word(0), word(1)), Noun::pair(word(2), word(3)))
}

#[test]
fn guest_intrinsics_import_exact_vm_sources_and_preserve_complete_values() {
    support::worker(|| {
        let dependencies = [
            (
                "vm.core.convert",
                include_str!("../../lib/vm/core/convert.tri"),
            ),
            ("vm.core.field", include_str!("../../lib/vm/core/field.tri")),
            ("vm.nox.noun", include_str!("../../lib/vm/nox/noun.tri")),
        ];
        let entry = "program sample use vm.core.convert use vm.core.field use vm.nox.noun fn main(input:Noun)->Noun{let d=noun.identity(input) let digest=noun.pair(noun.pair(noun.atom(d[0]),noun.atom(d[1])),noun.pair(noun.atom(d[2]),noun.atom(d[3]))) let copy=noun.pair(noun.head(input),vm.nox.noun.tail(input)) if noun.eq(input,copy){noun.pair(noun.atom(convert.as_field(vm.core.convert.as_u32(4294967295))),noun.pair(noun.atom(field.sub(0,1)),noun.pair(noun.atom(noun.as_field(noun.atom(7))),noun.pair(copy,digest))))}else{noun.atom(99)}}";
        let input = data::nested();
        let expected = Noun::pair(
            Atom(4294967295),
            Noun::pair(
                Atom(18446744069414584320),
                Noun::pair(Atom(7), Noun::pair(input.clone(), digest_words(&input))),
            ),
        );
        let modules = package(entry, &dependencies);
        let mut caps = data::caps();
        caps[9] = 3145728;
        let bytes = match support::try_compile_only_package(
            &modules,
            "sample",
            "main",
            support::options(),
            caps,
        )
        .unwrap()
        {
            Compilation::Program { bytes, .. } => bytes,
            other => panic!("{other:?}"),
        };
        for code in [&bytes, &seed(&modules).unwrap()] {
            assert_eq!(data::run(code, &input).unwrap().bytes, expected.encoded());
        }
        // The same exact sources include unused split/add/mul/neg/inv declarations.
        scalar("program sample use vm.core.convert use vm.core.field use vm.nox.noun fn main()->Field{field.sub(convert.as_field(convert.as_u32(9)),2)}", &dependencies, 7);
    });
}
