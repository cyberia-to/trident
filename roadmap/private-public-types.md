---
status: draft
author: mastercyb
area: cryptography
planned: 64K
---

# Private/Public Type Modifier for Zero-Knowledge Functions

## Motivation

Building zero-knowledge proofs manually requires constructing circuits: defining which inputs are witnesses (private), which are public inputs, and which are public outputs. This is circuit-level programming. The developer must understand the constraint system, manually split inputs into witness and public categories, and ensure that private values never leak into public positions.

This is exactly the kind of mechanical, error-prone work that compilers exist to eliminate. The `zk fn` modifier with `Private<T>` and `Public<T>` type annotations lets the developer express the privacy boundary at the language level. The compiler generates the witness/public-input split, validates that private values never flow into public outputs, and constructs the ZK-compatible TASM sequence. The developer writes normal Trident code; the compiler builds the zero-knowledge circuit.

## Design

### The `zk fn` Modifier

```trident
zk fn secret_transfer(
    amount:           Private<Field>,
    sender_balance:   Private<Field>,
    receiver_balance: Private<Field>,
) -> Public<(Commitment, Commitment)> {
    assert!(sender_balance >= amount);
    let new_sender   = sender_balance - amount;
    let new_receiver = receiver_balance + amount;
    (commit(new_sender), commit(new_receiver))
}
```

`Private<T>` marks a parameter as a witness input — it is provided to the prover but never revealed in the proof. `Public<T>` marks return values as public outputs — included in the proof and visible to the verifier. The function body is ordinary Trident code; the ZK structure is entirely in the type annotations.

### The Witness/Public-Input Split

When the compiler sees a `zk fn`, it generates:

1. **Witness section**: All `Private<T>` parameters are placed in the witness input section of the TASM program. They appear in the Processor table but are not committed to in the public proof transcript.
2. **Public input section**: Any explicitly `Public<T>` inputs appear here.
3. **Public output section**: Return values typed `Public<T>` are committed to in the proof transcript and included in the verification interface.

The developer provides values for `Private<T>` parameters when calling `prove()`. The verifier receives only the `Public<T>` outputs (and any `Public<T>` inputs). Private values are never transmitted.

### Taint Tracking for Confidentiality

The type checker performs taint analysis: `Private<T>` values are tainted. Any expression that depends on a tainted value is tainted. Tainted values cannot flow into `Public<T>` positions:

```trident
zk fn example(secret: Private<Field>) -> Public<Field> {
    secret + 1  // ERROR: private value flows into public output
}

zk fn correct_example(secret: Private<Field>) -> Public<Field> {
    hash(secret)  // OK: hash is a one-way function; output is public-safe
                  // (the type system trusts the developer here — hash does not
                  //  automatically sanitize; it is the developer's responsibility
                  //  to ensure the public output does not leak private information
                  //  beyond what is intended by the zero-knowledge property)
}
```

The taint system prevents accidental leakage — returning a private value directly, concatenating it into a public string, or including it in a public commitment without hashing. Intentional transformations (like `hash(secret)`) are permitted.

### Compilation to Zero-Knowledge TASM

The compiler wraps the `zk fn` body in TASM sequences that enforce the witness/public split at the virtual machine level. Triton VM's execution model distinguishes witness inputs from public inputs at the instruction level. The compiler maps:

- `Private<T>` parameters → `read_mem` from witness tape
- `Public<T>` outputs → explicit public output instructions

The STARK proof of the resulting TASM execution is a zero-knowledge proof: it proves that the function computed a valid output (the `Public<T>` return value) from some witness (the `Private<T>` inputs) without revealing the witness.

### The Developer Experience

Before `zk fn`: the developer writes a circuit description in a constraint DSL, manually manages witness/public splits, and validates privacy boundaries by code review.

After `zk fn`: the developer writes normal Trident, adds type annotations, and the compiler produces a verified ZK function. The difference is the difference between assembler and C.

```trident
// The developer thinks about the algorithm, not the circuit:
zk fn vote(
    choice:    Private<Candidate>,
    voter_key: Private<Field>,
) -> Public<VoteCommitment> {
    let commitment = commit(hash(voter_key) || encode(choice));
    commitment
}
// Compiler generates: witness split, taint check, ZK-compatible TASM
// Result: a zero-knowledge vote commitment provably computed from a valid voter+choice pair
```

## Key Tradeoffs

**Trust boundary at `hash`**: The taint system marks `hash(private_value)` as public-safe because the hash is a one-way function. But this assumes the output does not leak the input via other means (e.g., a length-leaking hash, or a hash collision). The type system cannot reason about cryptographic properties — it trusts the developer's choice of transformation.

**Private input validation**: A `zk fn` often needs to validate its private inputs (e.g., `sender_balance >= amount`). These validations generate STARK constraints that are part of the proof. The verifier learns that the assertion held — not what the values were. This is correct ZK behavior, but the developer must be careful not to validate in a way that leaks information (e.g., `assert!(amount == 42)` effectively reveals `amount`).

**Nested `zk fn` calls**: A `zk fn` that calls another `zk fn` requires composing the witness structures. The compiler handles this by treating the inner function's private inputs as part of the outer function's witness. The composed proof is still a single STARK proof — no recursive proof needed for simple composition.

**Performance**: Generating the witness requires executing the program with private inputs to produce the trace. This is the same as normal execution cost. There is no additional overhead from the ZK annotations themselves.

## Implementation Sketch

```rust
// typecheck/zk_types.rs
enum Privacy {
    Public,
    Private,  // tainted
}

fn taint_check(expr: &TirExpr, taint: &TaintMap) -> Result<Privacy, TaintError> {
    match expr {
        Var(id) => Ok(taint.privacy_of(*id)),
        BinOp(a, op, b) => {
            let pa = taint_check(a, taint)?;
            let pb = taint_check(b, taint)?;
            Ok(if pa == Private || pb == Private { Private } else { Public })
        }
        // Hash is the explicit public boundary:
        Hash(inner) => Ok(Public),  // developer's responsibility
        // ...
    }
}

// tir/zk_lowering.rs
fn lower_zk_function(func: &ZkFunction, tir: &mut TirBuilder) {
    // Emit witness read instructions for Private<T> parameters
    for param in func.private_params() {
        tir.emit(ReadWitness(param.id, param.ty));
    }
    // Lower body normally
    lower_body(&func.body, tir);
    // Emit public output instructions for Public<T> returns
    for ret in func.public_returns() {
        tir.emit(WritePublicOutput(ret.id, ret.ty));
    }
}
```
