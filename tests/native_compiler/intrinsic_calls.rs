use super::*;

#[test]
fn guest_intrinsic_purity_checks_source_member_before_lowering_identity() {
    support::worker(|| {
        let dependencies = [("ext.ops", "module ext.ops #[intrinsic(nox_noun_atom)] pub fn pub_read_atom(a:Field)->Noun pub fn wrapper()->Noun{pub_read_atom(7)}")];
        let entry =
            "program sample use ext.ops #[pure] fn main(input:Noun)->Noun{ops.pub_read_atom(7)}";
        reject_at(
            entry,
            &dependencies,
            "sample",
            "ops.pub_read_atom(7)",
            "ops.pub_read_atom",
            5,
        );
        assert!(seed(&package(entry, &dependencies)).is_err());
        agrees(
            "program sample use ext.ops #[pure] fn main(input:Noun)->Noun{ops.wrapper()}",
            &dependencies,
            &Atom(0),
            &Atom(7),
        );
    });
}

#[test]
fn guest_intrinsic_arguments_execute_once_and_in_source_order() {
    support::worker(|| {
        let dependencies = [("ext.ops", "module ext.ops #[intrinsic(sub)] pub fn difference(a:Field,b:Field)->Field #[intrinsic(nox_noun_pair)] pub fn join(a:Noun,b:Noun)->Noun")];
        for (body, expected, count) in [
            (
                "ops.join(nox_noun_atom(sub(9,2)),nox_noun_atom(sub(8,3)))",
                Noun::pair(Atom(7), Atom(5)),
                2,
            ),
            (
                "nox_noun_atom(ops.difference(sub(9,2),sub(8,3)))",
                Atom(2),
                3,
            ),
        ] {
            let entry = format!("program sample use ext.ops fn main(input:Noun)->Noun{{{body}}}");
            let bytes = agrees(&entry, &dependencies, &Atom(0), &expected);
            let mut trace = nox::trace::VecTrace::default();
            let run =
                data::run_traced(&bytes, &Atom(0), 1_000_000, 65536, 196608, &mut trace).unwrap();
            assert_eq!(run.bytes, expected.encoded());
            assert_eq!(trace.0.iter().filter(|row| row.col(0) == 6).count(), count);
        }
        let axis = "nox_noun_as_field(nox_noun_head(nox_noun_atom(1)))";
        let inv = "as_field(as_u32(4294967296))";
        for (first, second, trap) in [(axis, inv, "AxisError"), (inv, axis, "InvZero")] {
            for body in [
                format!("ops.join(nox_noun_atom({first}),nox_noun_atom({second}))"),
                format!("nox_noun_atom(ops.difference({first},{second}))"),
            ] {
                let entry =
                    format!("program sample use ext.ops fn main(input:Noun)->Noun{{{body}}}");
                let modules = package(&entry, &dependencies);
                for code in [program(&modules), seed(&modules).unwrap()] {
                    assert_eq!(
                        data::run(&code, &Atom(0)),
                        Err(format!("Error({trap})")),
                        "{entry}"
                    );
                }
            }
        }
        reject_at(
            "program sample use ext.ops fn main(input:Noun)->Noun{ops.join(input,7)}",
            &dependencies,
            "sample",
            "ops.join(input,7)",
            "7",
            5,
        );
    });
}
