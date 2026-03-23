// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use crate::ast::*;

use super::parse;

#[test]
fn test_minimal_program() {
    let file = parse("program test\n\nfn main() {\n}");
    assert_eq!(file.kind, FileKind::Program);
    assert_eq!(file.name.node, "test");
    assert_eq!(file.items.len(), 1);
}

#[test]
fn test_function_with_params() {
    let file = parse("program test\n\nfn add(a: Field, b: Field) -> Field {\n    a + b\n}");
    assert_eq!(file.items.len(), 1);
    if let Item::Fn(f) = &file.items[0].node {
        assert_eq!(f.name.node, "add");
        assert_eq!(f.params.len(), 2);
        assert!(f.return_ty.is_some());
    } else {
        panic!("expected function");
    }
}

#[test]
fn test_let_binding() {
    let file = parse("program test\n\nfn main() {\n    let a: Field = 42\n}");
    if let Item::Fn(f) = &file.items[0].node {
        let block = f.body.as_ref().unwrap();
        assert_eq!(block.node.stmts.len(), 1);
    }
}

#[test]
fn test_function_call() {
    let file = parse("program test\n\nfn main() {\n    let a: Field = pub_read()\n}");
    if let Item::Fn(f) = &file.items[0].node {
        let block = f.body.as_ref().unwrap();
        if let Stmt::Let { init, .. } = &block.node.stmts[0].node {
            assert!(matches!(init.node, Expr::Call { .. }));
        }
    }
}

#[test]
fn test_binary_expr() {
    let file = parse("program test\n\nfn main() {\n    let c: Field = a + b * c\n}");
    if let Item::Fn(f) = &file.items[0].node {
        let block = f.body.as_ref().unwrap();
        if let Stmt::Let { init, .. } = &block.node.stmts[0].node {
            // Should be Add(a, Mul(b, c)) due to precedence
            if let Expr::BinOp { op, .. } = &init.node {
                assert_eq!(*op, BinOp::Add);
            } else {
                panic!("expected binary op");
            }
        }
    }
}

#[test]
fn test_module() {
    let file = parse("module merkle\n\npub fn verify(root: Digest) {\n}");
    assert_eq!(file.kind, FileKind::Module);
    assert_eq!(file.name.node, "merkle");
    if let Item::Fn(f) = &file.items[0].node {
        assert!(f.is_pub);
    }
}

#[test]
fn test_program_declarations() {
    let file = parse("program test\n\npub input: [Field; 3]\npub output: Field\nsec input: [Field; 5]\n\nfn main() {\n}");
    assert_eq!(file.declarations.len(), 3);
    assert!(matches!(file.declarations[0], Declaration::PubInput(_)));
    assert!(matches!(file.declarations[1], Declaration::PubOutput(_)));
    assert!(matches!(file.declarations[2], Declaration::SecInput(_)));
}

#[test]
fn test_sec_ram_declaration() {
    let file =
        parse("program test\n\nsec ram: {\n    17: Field,\n    42: Field,\n}\n\nfn main() {\n}");
    assert_eq!(file.declarations.len(), 1);
    if let Declaration::SecRam(entries) = &file.declarations[0] {
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].0, 17);
        assert_eq!(entries[1].0, 42);
    } else {
        panic!("expected SecRam declaration");
    }
}

#[test]
fn test_tuple_destructure_let() {
    let file = parse(
        "program test\nfn main() {\n    let (a, b): (Field, Field) = (pub_read(), pub_read())\n}",
    );
    if let Item::Fn(f) = &file.items[0].node {
        let block = f.body.as_ref().unwrap();
        assert_eq!(block.node.stmts.len(), 1);
        if let Stmt::Let {
            pattern: Pattern::Tuple(names),
            ..
        } = &block.node.stmts[0].node
        {
            assert_eq!(names.len(), 2);
            assert_eq!(names[0].node, "a");
            assert_eq!(names[1].node, "b");
        } else {
            panic!("expected tuple destructuring let");
        }
    }
}

#[test]
fn test_event_declaration() {
    let file = parse("program test\nevent Transfer {\n    from: Field,\n    to: Field,\n    amount: Field,\n}\nfn main() {\n}");
    assert_eq!(file.items.len(), 2); // event + fn
    if let Item::Event(e) = &file.items[0].node {
        assert_eq!(e.name.node, "Transfer");
        assert_eq!(e.fields.len(), 3);
        assert_eq!(e.fields[0].name.node, "from");
        assert_eq!(e.fields[1].name.node, "to");
        assert_eq!(e.fields[2].name.node, "amount");
    } else {
        panic!("expected event declaration");
    }
}

#[test]
fn test_reveal_statement() {
    let file = parse("program test\nevent Ev { x: Field }\nfn main() {\n    let a: Field = pub_read()\n    reveal Ev { x: a }\n}");
    if let Item::Fn(f) = &file.items[1].node {
        let block = f.body.as_ref().unwrap();
        assert_eq!(block.node.stmts.len(), 2);
        if let Stmt::Reveal { event_name, fields } = &block.node.stmts[1].node {
            assert_eq!(event_name.node, "Ev");
            assert_eq!(fields.len(), 1);
            assert_eq!(fields[0].0.node, "x");
        } else {
            panic!("expected reveal statement");
        }
    }
}

#[test]
fn test_seal_statement() {
    let file = parse("program test\nevent Ev { x: Field, y: Field }\nfn main() {\n    seal Ev { x: pub_read(), y: pub_read() }\n}");
    if let Item::Fn(f) = &file.items[1].node {
        let block = f.body.as_ref().unwrap();
        assert_eq!(block.node.stmts.len(), 1);
        if let Stmt::Seal { event_name, fields } = &block.node.stmts[0].node {
            assert_eq!(event_name.node, "Ev");
            assert_eq!(fields.len(), 2);
        } else {
            panic!("expected seal statement");
        }
    }
}

#[test]
fn test_asm_basic() {
    let file = parse("program test\nfn main() {\n    asm { dup 0\n    add }\n}");
    if let Item::Fn(f) = &file.items[0].node {
        let block = f.body.as_ref().unwrap();
        assert_eq!(block.node.stmts.len(), 1);
        if let Stmt::Asm {
            body,
            effect,
            target,
        } = &block.node.stmts[0].node
        {
            assert!(body.contains("dup 0"));
            assert!(body.contains("add"));
            assert_eq!(*effect, 0);
            assert_eq!(*target, None);
        } else {
            panic!("expected asm statement");
        }
    }
}

#[test]
fn test_asm_with_effect() {
    let file = parse("program test\nfn main() {\n    asm(+1) { push 42 }\n}");
    if let Item::Fn(f) = &file.items[0].node {
        let block = f.body.as_ref().unwrap();
        if let Stmt::Asm { effect, .. } = &block.node.stmts[0].node {
            assert_eq!(*effect, 1);
        } else {
            panic!("expected asm statement");
        }
    }
}

#[test]
fn test_asm_between_statements() {
    let file = parse("program test\nfn main() {\n    let x: Field = pub_read()\n    asm { dup 0\nadd }\n    pub_write(x)\n}");
    if let Item::Fn(f) = &file.items[0].node {
        let block = f.body.as_ref().unwrap();
        assert_eq!(block.node.stmts.len(), 2);
        assert!(matches!(&block.node.stmts[0].node, Stmt::Let { .. }));
        assert!(matches!(&block.node.stmts[1].node, Stmt::Asm { .. }));
        assert!(block.node.tail_expr.is_some(), "pub_write(x) is tail expr");
    }
}

// --- cfg attribute parsing ---

#[test]
fn test_cfg_on_fn() {
    let file = parse("program test\n#[cfg(debug)]\nfn check() {}");
    if let Item::Fn(f) = &file.items[0].node {
        assert_eq!(f.cfg.as_ref().unwrap().node, "debug");
        assert_eq!(f.name.node, "check");
    } else {
        panic!("expected fn");
    }
}

#[test]
fn test_cfg_on_const() {
    let file = parse("program test\n#[cfg(release)]\nconst X: Field = 0");
    if let Item::Const(c) = &file.items[0].node {
        assert_eq!(c.cfg.as_ref().unwrap().node, "release");
        assert_eq!(c.name.node, "X");
    } else {
        panic!("expected const");
    }
}

#[test]
fn test_cfg_on_struct() {
    let file = parse("program test\n#[cfg(debug)]\nstruct Dbg { val: Field }");
    if let Item::Struct(s) = &file.items[0].node {
        assert_eq!(s.cfg.as_ref().unwrap().node, "debug");
        assert_eq!(s.name.node, "Dbg");
    } else {
        panic!("expected struct");
    }
}

#[test]
fn test_cfg_on_pub_fn() {
    let file = parse("program test\n#[cfg(release)]\npub fn fast() {}");
    if let Item::Fn(f) = &file.items[0].node {
        assert_eq!(f.cfg.as_ref().unwrap().node, "release");
        assert!(f.is_pub);
    } else {
        panic!("expected fn");
    }
}

#[test]
fn test_cfg_with_intrinsic() {
    let file = parse("module std.test\n#[cfg(debug)]\n#[intrinsic(add)]\npub fn add(a: Field, b: Field) -> Field");
    if let Item::Fn(f) = &file.items[0].node {
        assert_eq!(f.cfg.as_ref().unwrap().node, "debug");
        assert!(f.intrinsic.is_some());
    } else {
        panic!("expected fn");
    }
}

#[test]
fn test_no_cfg() {
    let file = parse("program test\nfn main() {}");
    if let Item::Fn(f) = &file.items[0].node {
        assert!(f.cfg.is_none());
    } else {
        panic!("expected fn");
    }
}

// --- match statement parsing ---

#[test]
fn test_match_basic() {
    let file = parse("program test\nfn main() {\n    let x: Field = pub_read()\n    match x {\n        0 => { pub_write(0) }\n        1 => { pub_write(1) }\n        _ => { pub_write(2) }\n    }\n}");
    if let Item::Fn(f) = &file.items[0].node {
        let block = f.body.as_ref().unwrap();
        assert_eq!(block.node.stmts.len(), 2);
        if let Stmt::Match { arms, .. } = &block.node.stmts[1].node {
            assert_eq!(arms.len(), 3);
            assert!(matches!(
                arms[0].pattern.node,
                MatchPattern::Literal(Literal::Integer(0))
            ));
            assert!(matches!(
                arms[1].pattern.node,
                MatchPattern::Literal(Literal::Integer(1))
            ));
            assert!(matches!(arms[2].pattern.node, MatchPattern::Wildcard));
        } else {
            panic!("expected match statement");
        }
    }
}

#[test]
fn test_match_bool_patterns() {
    let file = parse("program test\nfn main() {\n    let b: Bool = true\n    match b {\n        true => { pub_write(1) }\n        false => { pub_write(0) }\n    }\n}");
    if let Item::Fn(f) = &file.items[0].node {
        let block = f.body.as_ref().unwrap();
        if let Stmt::Match { arms, .. } = &block.node.stmts[1].node {
            assert_eq!(arms.len(), 2);
            assert!(matches!(
                arms[0].pattern.node,
                MatchPattern::Literal(Literal::Bool(true))
            ));
            assert!(matches!(
                arms[1].pattern.node,
                MatchPattern::Literal(Literal::Bool(false))
            ));
        } else {
            panic!("expected match statement");
        }
    }
}

#[test]
fn test_match_wildcard_only() {
    let file = parse("program test\nfn main() {\n    match pub_read() {\n        _ => { pub_write(0) }\n    }\n}");
    if let Item::Fn(f) = &file.items[0].node {
        let block = f.body.as_ref().unwrap();
        if let Stmt::Match { arms, .. } = &block.node.stmts[0].node {
            assert_eq!(arms.len(), 1);
            assert!(matches!(arms[0].pattern.node, MatchPattern::Wildcard));
        } else {
            panic!("expected match statement");
        }
    }
}

