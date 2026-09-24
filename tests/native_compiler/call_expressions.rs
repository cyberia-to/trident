use super::{
    signatures::{run_probe, Arena},
    support,
};
use nox::Order;
use support::data::Seq;

fn program(expression: &str) -> String {
    format!("program sample fn inner(a:Field,b:Field)->Field{{a*10+b}} fn outer(a:Field,b:Field)->Field{{a*100+b}} fn zero()->Field{{7}} fn flag()->Bool{{true}} fn unit(){{}} fn main()->Field{{{expression}}}")
}

fn parse(expression: &str) -> Result<(u64, u64), u64> {
    static PROBE: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    let code = PROBE.get_or_init(|| {
        trident::compile_native_artifact_project(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/native_function_expression.tri"),
            &trident::CompileOptions::default(),
            trident::NativeArtifactProfile::RawNoun,
            trident::NATIVE_ARTIFACT_LIMITS,
        )
        .unwrap()
        .bytes
    });
    let (mut arena, result) = run_probe(&program(expression), 4096, code);
    let code = scalar(&arena, arena.head(result).unwrap());
    if code != 0 {
        return Err(code);
    }
    let payload = arena.tail(result).unwrap();
    let nodes = arena.head(payload).unwrap();
    let rest = arena.tail(payload).unwrap();
    let arguments = arena.head(rest).unwrap();
    let final_id = scalar(&arena, arena.tail(rest).unwrap()) as u32;
    let nodes = Seq::decode(&mut arena, nodes, 4096, 1_000_000).unwrap();
    let arguments = Seq::decode(&mut arena, arguments, 4096, 1_000_000).unwrap();
    let record = nodes.get(&arena, final_id).unwrap();
    let ty = scalar(&arena, arena.head(arena.tail(record).unwrap()).unwrap());
    Ok((interpret(&arena, nodes, arguments, final_id), ty))
}

fn scalar(arena: &Arena, value: Order) -> u64 {
    arena.atom_value(value).unwrap().as_u64()
}

// Independent small interpreter: argument links are owned lists, not ranges.
// The helper arithmetic below is fixed by the source corpus, never generated code.
fn interpret(arena: &Arena, nodes: Seq, arguments: Seq, id: u32) -> u64 {
    let node = nodes.get(arena, id).unwrap();
    let value = arena.head(node).unwrap();
    let kind = scalar(arena, arena.head(value).unwrap());
    let operands = arena.tail(value).unwrap();
    let a = scalar(arena, arena.head(operands).unwrap());
    let b = scalar(arena, arena.tail(operands).unwrap());
    match kind {
        0 | 2 => a,
        3 => {
            interpret(arena, nodes, arguments, a as u32)
                + interpret(arena, nodes, arguments, b as u32)
        }
        4 => {
            interpret(arena, nodes, arguments, a as u32)
                * interpret(arena, nodes, arguments, b as u32)
        }
        5 => u64::from(
            interpret(arena, nodes, arguments, a as u32)
                != interpret(arena, nodes, arguments, b as u32),
        ),
        6 => {
            let mut link = b as u32;
            let mut values = Vec::new();
            while link != 4096 {
                let argument = arguments.get(arena, link).unwrap();
                let expr = scalar(arena, arena.head(argument).unwrap()) as u32;
                assert!(expr < id, "call operands precede their parent");
                values.push(interpret(arena, nodes, arguments, expr));
                link = scalar(arena, arena.tail(argument).unwrap()) as u32;
                assert!(values.len() <= 6);
            }
            values.reverse();
            match a {
                0 => {
                    assert_eq!(values.len(), 2);
                    values[0] * 10 + values[1]
                }
                1 => {
                    assert_eq!(values.len(), 2);
                    values[0] * 100 + values[1]
                }
                2 => {
                    assert!(values.is_empty());
                    7
                }
                3 | 4 => {
                    assert!(values.is_empty());
                    0
                }
                _ => panic!("unknown helper {a}"),
            }
        }
        _ => panic!("unexpected expression kind {kind}"),
    }
}

#[test]
fn nested_calls_own_arguments_and_preserve_expression_types_and_precedence() {
    support::worker(|| {
        for (expression, value, ty) in [
            ("outer(inner(1,2),inner(3,4))", 1234, 0),
            ("outer(inner(1,2,),inner(3,4),)", 1234, 0),
            ("1+inner(2,3)*4", 93, 0),
            ("inner((1+2),(3+4))", 37, 0),
            ("inner(zero(),zero())", 77, 0),
            ("zero()+zero()", 14, 0),
            ("flag()==true", 0, 1),
            ("unit()==unit()", 0, 1),
        ] {
            assert_eq!(parse(expression), Ok((value, ty)), "{expression}");
        }
    });
}

#[test]
fn calls_reject_unknown_names_arity_types_and_unbalanced_argument_delimiters() {
    support::worker(|| {
        for expression in [
            "missing()",
            "inner()",
            "inner(1)",
            "inner(1,2,3)",
            "inner(1,true)",
            "inner(1,unit())",
            "zero(1)",
            "unit()+1",
        ] {
            assert_eq!(parse(expression), Err(5), "{expression}");
        }
        for expression in [
            "inner(,1)",
            "inner(1,,2)",
            "inner(1 2)",
            "inner((1,2))",
            "inner(1,2+)",
            "inner(1,2",
            "1+inner(1 2)",
        ] {
            assert_eq!(parse(expression), Err(2), "{expression}");
        }
    });
}

#[test]
fn nested_calls_and_mixed_parentheses_cross_parser_chunks() {
    support::worker(|| {
        for count in [7, 8, 9] {
            let expression = format!("{}1{}", "inner(0,".repeat(count), ")".repeat(count));
            let expected = if count <= 64 { Ok((1, 0)) } else { Err(7) };
            assert_eq!(parse(&expression), expected, "calls={count}");
        }
        for count in [6, 7, 8] {
            let expression = format!("inner(0,{}1{})", "(".repeat(count), ")".repeat(count));
            let expected = if count < 64 { Ok((1, 0)) } else { Err(7) };
            assert_eq!(parse(&expression), expected, "parentheses={count}");
        }
    });
}
