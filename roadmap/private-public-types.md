---
status: draft
author: mastercyb
area: cryptography
planned: 64K
---

# Private/Public Type Modifier for Zero-Knowledge Functions

**Related:** [[commitment-syntax]] · [[contracts]] · [[linear-types-crypto]] · [[bbg]] · [[cybergraph]] · [[nox]] · [[zheng]]

## Vision

[[bbg]]'s core privacy guarantee — individual contributions private, aggregate publicly verifiable — finds its language-level expression here. A neuron contributes a private vote: `zk fn vote(choice: Private<Candidate>, voter_key: Private<Field>) -> Public<VoteCommitment>`. The aggregate vote count is [[cybergraph]]-visible; no individual ballot ever appears on-chain.

The neural network running in Trident takes `Private<[Field; 16]>` (private inputs) and returns `Public<Field>` (public class label). The [[cybergraph]] records the public output and its [[zheng]] proof via a cyberlink from the inference request to the result. Individual inputs never appear on-chain. This is the architecture for AGI that respects individual sovereignty while enabling collective intelligence.

`Private<T>` values are injected via [[nox]]'s call mechanism (Layer 2, pattern 16, `CallProvider::provide()`). The `seal` event in language.md §10 commits a `Private<T>` value into the [[cybergraph]] without revealing it — the commitment particle exists in the graph as a hemera-addressed node. The `reveal` event publishes the `Public<T>` output with its [[zheng]] proof, creating a cyberlink from the computation request to the proven result. The privacy boundary is enforced by the VM, not by convention.

## Motivation

Building zero-knowledge proofs manually requires constructing circuits: defining which inputs are witnesses (private), which are public inputs, and which are public outputs. This is circuit-level programming. The developer must understand the constraint system, manually split inputs into witness and public categories, and ensure that private values never leak into public positions.

This is exactly the kind of mechanical, error-prone work that compilers exist to eliminate. The `zk fn` modifier with `Private<T>` and `Public<T>` type annotations lets the developer express the privacy boundary at the language level. The compiler generates the witness/public-input split, validates that private values never flow into public outputs, and constructs the ZK-compatible nox pattern sequence. The developer writes normal Trident code; the compiler builds the zero-knowledge circuit.

### Connection to nox's Call Mechanism

The `Private<T>` type maps directly to nox's Layer 2 call mechanism (pattern 16). In nox, `CallProvider::provide()` injects witness atoms into the trace — these are the private inputs. They are verified by Layer 1 nox constraints but never appear in the public proof transcript. `Private<T>` is syntactic sugar over this mechanism: the compiler generates a `CallProvider::provide()` call for each `Private<T>` parameter and wraps it in nox constraints that verify the witness satisfies the function's `#[requires]` conditions (see [[contracts]]).

The call pattern is the privacy boundary in nox. Layer 2 provides; Layer 1 constrains. The zheng proof certifies that the constraints were satisfied without revealing what was provided.

### Connection to `seal`/`reveal` Events

Language.md §10 already has two commitment events built into the language:
- `reveal` — writes all fields to public output (visible to verifier)
- `seal` — hashes all fields via the sponge; only the commitment digest is public

`Public<T>` return values compile to `reveal` events. `Private<T>` values that need a public commitment (e.g., for cross-function verification) use `seal` internally. See [[commitment-syntax]] for the full commitment primitive design built on these events.

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

1. **Witness section**: All `Private<T>` parameters are placed in the witness input section of the nox execution. They appear in the nox trace (as Layer 2 call atoms) but are not committed to in the public proof transcript.
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

### Compilation to Zero-Knowledge nox Patterns

The compiler wraps the `zk fn` body in nox patterns that enforce the witness/public split at the VM level. nox's execution model distinguishes witness inputs (Layer 2 call atoms injected via `CallProvider::provide()`) from public inputs at the pattern level. The compiler maps:

- `Private<T>` parameters → nox Layer 2 call injection (`CallProvider::provide()`)
- `Public<T>` outputs → `reveal` events (language.md §10), writing to public output
- `Private<T>` values needing commitment → `seal` events (language.md §10)

The zheng proof of the resulting nox trace is a zero-knowledge proof: it proves that the function computed a valid output (the `Public<T>` return value) from some witness (the `Private<T>` call inputs) without revealing the witness. Layer 1 nox constraints verify the witness was well-formed; zheng certifies the constraints held.

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
// Compiler generates: call injection, taint check, ZK-compatible nox patterns
// Result: a zero-knowledge vote commitment provably computed from a valid voter+choice pair
```

## Key Tradeoffs

**Trust boundary at `hash`**: The taint system marks `hash(private_value)` as public-safe because the hash is a one-way function. But this assumes the output does not leak the input via other means (e.g., a length-leaking hash, or a hash collision). The type system cannot reason about cryptographic properties — it trusts the developer's choice of transformation.

**Private input validation**: A `zk fn` often needs to validate its private inputs (e.g., `sender_balance >= amount`). These validations generate nox constraints (Layer 1) that are part of the zheng proof. The verifier learns that the assertion held — not what the values were. This is correct ZK behavior, but the developer must be careful not to validate in a way that leaks information (e.g., `assert!(amount == 42)` effectively reveals `amount`).

**Nested `zk fn` calls**: A `zk fn` that calls another `zk fn` requires composing the witness structures. The compiler handles this by treating the inner function's private inputs as part of the outer function's witness (flattened call injection). The composed proof is still a single zheng proof of the nox trace — no recursive proof needed for simple composition.

**Performance**: Generating the witness requires executing the program with private inputs to produce the nox trace. This is the same as normal execution cost. There is no additional overhead from the ZK annotations themselves.

**Proof composition for complex ZK**: For ZK functions that need to verify other proofs (e.g., proving knowledge of a valid inner computation), use `proof_block` (language.md §17, Tier 3). `Private<T>`/`Public<T>` handles the single-level case; `proof_block` handles the recursive case.

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
    // Emit nox Layer 2 call injection for Private<T> parameters
    for param in func.private_params() {
        tir.emit(HintProvide(param.id, param.ty));  // CallProvider::provide()
    }
    // Lower body normally
    lower_body(&func.body, tir);
    // Emit reveal events for Public<T> returns (language.md §10)
    for ret in func.public_returns() {
        tir.emit(RevealEvent(ret.id, ret.ty));
    }
}
```
