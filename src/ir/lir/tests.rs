// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use super::*;

#[test]
fn test_reg_display() {
    assert_eq!(format!("{}", Reg(0)), "v0");
    assert_eq!(format!("{}", Reg(42)), "v42");
}

#[test]
fn test_label_display() {
    assert_eq!(format!("{}", Label::new("loop_1")), "loop_1");
}

#[test]
fn test_reg_equality() {
    assert_eq!(Reg(0), Reg(0));
    assert_ne!(Reg(0), Reg(1));
}

#[test]
fn test_label_equality() {
    assert_eq!(Label::new("a"), Label::new("a"));
    assert_ne!(Label::new("a"), Label::new("b"));
}

#[test]
fn test_lirop_display() {
    let r0 = Reg(0);
    let r1 = Reg(1);
    let r2 = Reg(2);

    assert_eq!(format!("{}", LIROp::LoadImm(r0, 42)), "li v0, 42");
    assert_eq!(format!("{}", LIROp::Add(r0, r1, r2)), "add v0, v1, v2");
    assert_eq!(format!("{}", LIROp::Move(r0, r1)), "mv v0, v1");
    assert_eq!(format!("{}", LIROp::Call("main".into())), "call main");
    assert_eq!(format!("{}", LIROp::Return), "ret");
}

#[test]
fn test_lirop_branch_display() {
    let op = LIROp::Branch {
        cond: Reg(0),
        if_true: Label::new("then"),
        if_false: Label::new("else"),
    };
    assert_eq!(format!("{}", op), "br v0, then, else");
}

#[test]
fn test_lirop_memory_display() {
    assert_eq!(
        format!(
            "{}",
            LIROp::Load {
                dst: Reg(0),
                base: Reg(1),
                offset: 8
            }
        ),
        "ld v0, [v1+8]"
    );
    assert_eq!(
        format!(
            "{}",
            LIROp::Store {
                src: Reg(0),
                base: Reg(1),
                offset: 0
            }
        ),
        "st v0, [v1+0]"
    );
}

#[test]
fn test_lirop_all_variants_construct() {
    let r0 = Reg(0);
    let r1 = Reg(1);
    let r2 = Reg(2);
    let r3 = Reg(3);
    let _ops: Vec<LIROp> = vec![
        // Tier 0
        LIROp::Call("f".into()),
        LIROp::Return,
        LIROp::Halt,
        LIROp::Branch {
            cond: r0,
            if_true: Label::new("t"),
            if_false: Label::new("f"),
        },
        LIROp::Jump(Label::new("x")),
        LIROp::LabelDef(Label::new("x")),
        LIROp::FnStart("main".into()),
        LIROp::FnEnd,
        LIROp::Entry("main".into()),
        LIROp::Comment("test".into()),
        LIROp::Asm {
            lines: vec!["nop".into()],
        },
        // Tier 1
        LIROp::LoadImm(r0, 0),
        LIROp::Move(r0, r1),
        LIROp::Add(r0, r1, r2),
        LIROp::Mul(r0, r1, r2),
        LIROp::Eq(r0, r1, r2),
        LIROp::Lt(r0, r1, r2),
        LIROp::And(r0, r1, r2),
        LIROp::Or(r0, r1, r2),
        LIROp::Xor(r0, r1, r2),
        LIROp::DivMod {
            dst_quot: r0,
            dst_rem: r1,
            src1: r2,
            src2: r3,
        },
        LIROp::Shl(r0, r1, r2),
        LIROp::Shr(r0, r1, r2),
        LIROp::Invert(r0, r1),
        LIROp::Split {
            dst_hi: r0,
            dst_lo: r1,
            src: r2,
        },
        LIROp::Log2(r0, r1),
        LIROp::Pow(r0, r1, r2),
        LIROp::PopCount(r0, r1),
        LIROp::ReadIo { dst: r0, count: 1 },
        LIROp::WriteIo { src: r0, count: 1 },
        LIROp::Hint { dst: r0, count: 1 },
        LIROp::Load {
            dst: r0,
            base: r1,
            offset: 0,
        },
        LIROp::Store {
            src: r0,
            base: r1,
            offset: 0,
        },
        LIROp::LoadMulti {
            dst: r0,
            base: r1,
            width: 4,
        },
        LIROp::StoreMulti {
            src: r0,
            base: r1,
            width: 4,
        },
        LIROp::Assert { src: r0, count: 1 },
        LIROp::Assert { src: r0, count: 4 },
        LIROp::Hash {
            dst: r0,
            src: r1,
            count: 1,
        },
        LIROp::Reveal {
            name: "Transfer".into(),
            tag: 0,
            src: r0,
            field_count: 2,
        },
        LIROp::Seal {
            name: "Nullifier".into(),
            tag: 1,
            src: r0,
            field_count: 1,
        },
        LIROp::RamRead {
            dst: r0,
            key: r1,
            width: 1,
        },
        LIROp::RamWrite {
            key: r0,
            src: r1,
            width: 1,
        },
        // Tier 2
        LIROp::SpongeInit(r0),
        LIROp::SpongeAbsorb { state: r0, src: r1 },
        LIROp::SpongeSqueeze { dst: r0, state: r1 },
        LIROp::SpongeLoad {
            state: r0,
            addr: r1,
        },
        LIROp::MerkleStep {
            dst: r0,
            node: r1,
            sibling: r2,
        },
        LIROp::MerkleLoad {
            dst: r0,
            node: r1,
            addr: r2,
        },
        // Tier 3
        LIROp::ExtMul(r0, r1, r2),
        LIROp::ExtInvert(r0, r1),
        LIROp::FoldExt {
            dst: r0,
            src1: r1,
            src2: r2,
        },
        LIROp::FoldBase {
            dst: r0,
            src1: r1,
            src2: r2,
        },
        LIROp::ProofBlock {
            program_hash: "abc123".into(),
        },
        LIROp::ProofBlockEnd,
    ];
}
