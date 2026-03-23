// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use crate::*;

#[test]
fn test_recursive_verifier_compiles() {
    let path = std::path::Path::new("os/neptune/programs/recursive_verifier.tri");
    if !path.exists() {
        return; // skip if running from different cwd
    }
    let result = compile_project(path);
    assert!(
        result.is_ok(),
        "recursive verifier should compile: {:?}",
        result.err()
    );
    let tasm = result.unwrap();
    assert!(
        tasm.contains("xx_dot_step"),
        "should emit xx_dot_step instruction"
    );
}

#[test]
fn test_xfield_dot_step_intrinsics() {
    let dir = tempfile::tempdir().unwrap();
    // Write the entry program that uses xx_dot_step via os.neptune.xfield
    let main_path = dir.path().join("main.tri");
    std::fs::write(
        &main_path,
        r#"program test
use os.neptune.xfield

fn main() {
let ptr_a: Field = divine()
let ptr_b: Field = divine()
let result: Digest = xfield.xx_dot_step(0, 0, 0, ptr_a, ptr_b)
let (r0, r1, r2, r3, r4) = result
pub_write(r0)
}
"#,
    )
    .unwrap();
    // Create os/neptune directory and copy xfield.tri
    let ext_dir = dir.path().join("os").join("neptune");
    std::fs::create_dir_all(&ext_dir).unwrap();
    std::fs::copy("os/neptune/xfield.tri", ext_dir.join("xfield.tri")).unwrap_or_default();

    let result = compile_project(&main_path);
    assert!(
        result.is_ok(),
        "xx_dot_step intrinsic should compile: {:?}",
        result.err()
    );
    let tasm = result.unwrap();
    assert!(
        tasm.contains("xx_dot_step"),
        "emitted TASM should contain xx_dot_step"
    );
}

#[test]
fn test_xb_dot_step_intrinsic() {
    let dir = tempfile::tempdir().unwrap();
    let main_path = dir.path().join("main.tri");
    std::fs::write(
        &main_path,
        r#"program test
use os.neptune.xfield

fn main() {
let ptr_a: Field = divine()
let ptr_b: Field = divine()
let result: Digest = xfield.xb_dot_step(0, 0, 0, ptr_a, ptr_b)
let (r0, r1, r2, r3, r4) = result
pub_write(r0)
}
"#,
    )
    .unwrap();
    let ext_dir = dir.path().join("os").join("neptune");
    std::fs::create_dir_all(&ext_dir).unwrap();
    std::fs::copy("os/neptune/xfield.tri", ext_dir.join("xfield.tri")).unwrap_or_default();

    let result = compile_project(&main_path);
    assert!(
        result.is_ok(),
        "xb_dot_step intrinsic should compile: {:?}",
        result.err()
    );
    let tasm = result.unwrap();
    assert!(
        tasm.contains("xb_dot_step"),
        "emitted TASM should contain xb_dot_step"
    );
}

#[test]
fn test_xfe_inner_product_library() {
    let dir = tempfile::tempdir().unwrap();
    let main_path = dir.path().join("main.tri");
    std::fs::write(
        &main_path,
        r#"program test
use os.neptune.recursive

fn main() {
let ptr_a: Field = divine()
let ptr_b: Field = divine()
let count: Field = divine()
let result: Digest = recursive.xfe_inner_product(ptr_a, ptr_b, count)
let (r0, r1, r2, r3, r4) = result
pub_write(r0)
pub_write(r1)
pub_write(r2)
}
"#,
    )
    .unwrap();
    // Copy library files
    let ext_dir = dir.path().join("os").join("neptune");
    std::fs::create_dir_all(&ext_dir).unwrap();
    std::fs::copy("os/neptune/xfield.tri", ext_dir.join("xfield.tri")).unwrap_or_default();
    std::fs::copy("os/neptune/recursive.tri", ext_dir.join("recursive.tri"))
        .unwrap_or_default();
    // Copy vm files that recursive.tri depends on
    let vm_io = dir.path().join("vm").join("io");
    let vm_core = dir.path().join("vm").join("core");
    std::fs::create_dir_all(&vm_io).unwrap();
    std::fs::create_dir_all(&vm_core).unwrap();
    std::fs::copy("vm/io/io.tri", vm_io.join("io.tri")).unwrap_or_default();
    std::fs::copy("vm/core/assert.tri", vm_core.join("assert.tri")).unwrap_or_default();

    let result = compile_project(&main_path);
    assert!(
        result.is_ok(),
        "xfe_inner_product should compile: {:?}",
        result.err()
    );
    let tasm = result.unwrap();
    assert!(
        tasm.contains("xx_dot_step"),
        "inner product should use xx_dot_step"
    );
}

#[test]
fn test_xb_inner_product_library() {
    let dir = tempfile::tempdir().unwrap();
    let main_path = dir.path().join("main.tri");
    std::fs::write(
        &main_path,
        r#"program test
use os.neptune.recursive

fn main() {
let ptr_a: Field = divine()
let ptr_b: Field = divine()
let count: Field = divine()
let result: Digest = recursive.xb_inner_product(ptr_a, ptr_b, count)
let (r0, r1, r2, r3, r4) = result
pub_write(r0)
}
"#,
    )
    .unwrap();
    let ext_dir = dir.path().join("os").join("neptune");
    std::fs::create_dir_all(&ext_dir).unwrap();
    std::fs::copy("os/neptune/xfield.tri", ext_dir.join("xfield.tri")).unwrap_or_default();
    std::fs::copy("os/neptune/recursive.tri", ext_dir.join("recursive.tri"))
        .unwrap_or_default();
    let vm_io = dir.path().join("vm").join("io");
    let vm_core = dir.path().join("vm").join("core");
    std::fs::create_dir_all(&vm_io).unwrap();
    std::fs::create_dir_all(&vm_core).unwrap();
    std::fs::copy("vm/io/io.tri", vm_io.join("io.tri")).unwrap_or_default();
    std::fs::copy("vm/core/assert.tri", vm_core.join("assert.tri")).unwrap_or_default();

    let result = compile_project(&main_path);
    assert!(
        result.is_ok(),
        "xb_inner_product should compile: {:?}",
        result.err()
    );
    let tasm = result.unwrap();
    assert!(
        tasm.contains("xb_dot_step"),
        "xb inner product should use xb_dot_step"
    );
}

#[test]
fn test_proof_composition_library() {
    let dir = tempfile::tempdir().unwrap();
    let main_path = dir.path().join("main.tri");
    std::fs::write(
        &main_path,
        r#"program test
use os.neptune.proof

fn main() {
proof.verify_inner_proof(4)
}
"#,
    )
    .unwrap();
    // Copy all required library files
    let ext_dir = dir.path().join("os").join("neptune");
    std::fs::create_dir_all(&ext_dir).unwrap();
    std::fs::copy("os/neptune/proof.tri", ext_dir.join("proof.tri")).unwrap_or_default();
    std::fs::copy("os/neptune/recursive.tri", ext_dir.join("recursive.tri"))
        .unwrap_or_default();
    std::fs::copy("os/neptune/xfield.tri", ext_dir.join("xfield.tri")).unwrap_or_default();
    let vm_io = dir.path().join("vm").join("io");
    let vm_core = dir.path().join("vm").join("core");
    std::fs::create_dir_all(&vm_io).unwrap();
    std::fs::create_dir_all(&vm_core).unwrap();
    std::fs::copy("vm/io/io.tri", vm_io.join("io.tri")).unwrap_or_default();
    std::fs::copy("vm/core/assert.tri", vm_core.join("assert.tri")).unwrap_or_default();

    let result = compile_project(&main_path);
    assert!(
        result.is_ok(),
        "proof composition should compile: {:?}",
        result.err()
    );
    let tasm = result.unwrap();
    assert!(
        tasm.contains("xx_dot_step"),
        "should use xx_dot_step for inner products"
    );
}

#[test]
fn test_proof_aggregation() {
    let dir = tempfile::tempdir().unwrap();
    let main_path = dir.path().join("main.tri");
    std::fs::write(
        &main_path,
        r#"program test
use os.neptune.proof

fn main() {
let n: Field = pub_read()
proof.aggregate_proofs(n, 4)
}
"#,
    )
    .unwrap();
    let ext_dir = dir.path().join("os").join("neptune");
    std::fs::create_dir_all(&ext_dir).unwrap();
    std::fs::copy("os/neptune/proof.tri", ext_dir.join("proof.tri")).unwrap_or_default();
    std::fs::copy("os/neptune/recursive.tri", ext_dir.join("recursive.tri"))
        .unwrap_or_default();
    std::fs::copy("os/neptune/xfield.tri", ext_dir.join("xfield.tri")).unwrap_or_default();
    let vm_io = dir.path().join("vm").join("io");
    let vm_core = dir.path().join("vm").join("core");
    std::fs::create_dir_all(&vm_io).unwrap();
    std::fs::create_dir_all(&vm_core).unwrap();
    std::fs::copy("vm/io/io.tri", vm_io.join("io.tri")).unwrap_or_default();
    std::fs::copy("vm/core/assert.tri", vm_core.join("assert.tri")).unwrap_or_default();

    let result = compile_project(&main_path);
    assert!(
        result.is_ok(),
        "proof aggregation should compile: {:?}",
        result.err()
    );
    let tasm = result.unwrap();
    assert!(
        tasm.contains("xx_dot_step"),
        "aggregation should use xx_dot_step"
    );
}

#[test]
fn test_proof_relay_example_compiles() {
    let path = std::path::Path::new("os/neptune/programs/proof_relay.tri");
    if !path.exists() {
        return;
    }
    let result = compile_project(path);
    assert!(
        result.is_ok(),
        "proof relay example should compile: {:?}",
        result.err()
    );
}

#[test]
fn test_proof_aggregator_example_compiles() {
    let path = std::path::Path::new("os/neptune/programs/proof_aggregator.tri");
    if !path.exists() {
        return;
    }
    let result = compile_project(path);
    assert!(
        result.is_ok(),
        "proof aggregator example should compile: {:?}",
        result.err()
    );
}

#[test]
fn test_transaction_validation_compiles() {
    let path = std::path::Path::new("os/neptune/programs/transaction_validation.tri");
    if !path.exists() {
        return;
    }
    let result = compile_project(path);
    assert!(
        result.is_ok(),
        "transaction validation should compile: {:?}",
        result.err()
    );
    let tasm = result.unwrap();
    assert!(
        tasm.contains("xx_dot_step"),
        "should use recursive verification"
    );
    assert!(
        tasm.contains("merkle_step"),
        "should authenticate kernel fields"
    );
}

#[test]
fn test_neptune_lock_scripts_compile() {
    for name in &["generation", "symmetric", "multisig", "timelock"] {
        let path_str = format!("os/neptune/locks/{}.tri", name);
        let path = std::path::Path::new(&path_str);
        if !path.exists() {
            continue;
        }
        let result = compile_project(path);
        assert!(
            result.is_ok(),
            "{} should compile: {:?}",
            name,
            result.err()
        );
    }
}

#[test]
fn test_neptune_type_scripts_compile() {
    for name in &["native_currency", "custom_token"] {
        let path_str = format!("os/neptune/types/{}.tri", name);
        let path = std::path::Path::new(&path_str);
        if !path.exists() {
            continue;
        }
        let result = compile_project(path);
        assert!(
            result.is_ok(),
            "{} should compile: {:?}",
            name,
            result.err()
        );
    }
}

