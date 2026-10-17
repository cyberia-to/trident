//! Encode the exact original dynamic-apply probe for the structured CLI.
//! Construction uses the existing parser and codec, with no evaluator call.
use joy_rs::formula;
use nox::{artifact, Reduction};
use std::{fs::OpenOptions, io::Write, path::Path};

fn run() -> Result<(), String> {
    let destination = std::env::args().nth(1).ok_or("output directory required")?;
    let mut arena = Reduction::<4096>::new();
    let original = "[2 [[1 0] [3 [[0 2] [1 42]]]]]";
    // ART1 = [tag [machine [input-profile [output-profile [formula 0]]]]].
    let source = format!("[1095914545 [0 [0 [0 [{original} 0]]]]]");
    let program = formula::parse(&mut arena, &source)?;
    let input = formula::build_subject(&mut arena, &[1])?;
    let expected = formula::parse(&mut arena, "42")?;
    for (name, root) in [
        ("program.dag", program),
        ("input.dag", input),
        ("expected.dag", expected),
    ] {
        let encoded = artifact::encode(
            &arena,
            root,
            artifact::Limits {
                max_bytes: 16384,
                max_nodes: 1024,
                max_depth: 256,
            },
        )
        .map_err(|e| format!("encode {name}: {e:?}"))?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(Path::new(&destination).join(name))
            .map_err(|e| e.to_string())?;
        file.write_all(&encoded).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn main() -> Result<(), String> {
    run()
}
