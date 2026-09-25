use super::{nouns::data, support};
use data::Noun::{self, Atom};

#[test]
fn array_elements_base_and_index_execute_once_in_source_order() {
    support::worker(|| {
        let source="program sample fn make()->[Field;2]{[sub(9,2),sub(8,3)]} fn index()->Field{sub(1,1)} fn main(input:Noun)->Noun{nox_noun_atom(make()[index()])}";
        let code = data::compile(source);
        let mut trace = nox::trace::VecTrace::default();
        let run = data::run_traced(&code, &Atom(0), 1_000_000, 65536, 196608, &mut trace).unwrap();
        assert_eq!(run.bytes, Atom(7).encoded());
        assert_eq!(trace.0.iter().filter(|r| r.col(0) == 6).count(), 3);
        let axis = "nox_noun_as_field(nox_noun_head(nox_noun_atom(1)))";
        let inv = "as_field(as_u32(4294967296))";
        for (body, error) in [
            (format!("let a=[{axis},{inv}] input"), "AxisError"),
            (format!("let a=[{inv},{axis}] input"), "InvZero"),
            (format!("nox_noun_atom([{axis}][{inv}])"), "AxisError"),
            (format!("nox_noun_atom([7][{inv}])"), "InvZero"),
            (
                format!("let unused=[7][nox_noun_as_field(input)] input"),
                "InvZero",
            ),
        ] {
            let source = data::source(&body);
            for bytes in [data::compile(&source), data::seed(&source)] {
                assert_eq!(
                    data::run(&bytes, &Atom(1)),
                    Err(format!("Error({error})")),
                    "{source}"
                );
            }
        }
        let values = (0..33).map(|i| i.to_string()).collect::<Vec<_>>().join(",");
        let source=data::source(&format!("let a=[{values}] let mut out=nox_noun_atom(0) for i in 0..33{{out=nox_noun_pair(nox_noun_atom(a[i]),out)}} out"));
        let mut expected = Atom(0);
        for i in 0..33 {
            expected = Noun::pair(Atom(i), expected);
        }
        data::agrees(&source, &Atom(0), &expected);
    });
}
