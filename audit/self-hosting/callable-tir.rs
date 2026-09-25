//! Execute the direct TIR exported by the compiler's checked fixture test.
use trident::tir::{TIROp, TIROp::*};
use triton_vm::prelude::*;
include!(env!("CALLABLE_TIR_FIXTURES_RS"));
fn main() {
    for (name, ir, expected) in fixtures() {
        let assembly = trisha_rs::lower::lower_checked(&ir)
            .expect("legal TIR")
            .join("\n");
        let program = Program::from_code(&assembly).expect("legal owner assembly");
        let actual: Vec<_> = VM::run(program, PublicInput::default(), NonDeterminism::default())
            .expect("direct TIR executes")
            .into_iter()
            .map(|v| v.value())
            .collect();
        assert_eq!(actual, expected, "{name}");
        println!(
            "{}",
            serde_json::json!({"case":name,"expected":expected,"actual":actual})
        );
    }
}
