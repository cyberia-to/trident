---
status: draft
author: mastercyb
area: type system
planned: 32K
---

# Linear Types for Cryptographic Values

**Related proposals:** [[refinement-types]], [[private-public-types]], [[contracts]]
**Reference:** [language.md §11 — Type Checking Rules](../reference/language.md)

## Motivation

Cryptographic hygiene failures are not runtime bugs. They are type errors that the type system fails to catch. A nonce used twice breaks security. A witness accessed twice leaks private data. A secret key copied into a public context reveals it. These errors are systematic, they occur across all cryptographic codebases, and they are invisible to conventional type systems because conventional type systems track data type — not data usage count.

Rust's borrow checker solved a closely related problem for memory safety: a value can be moved (consumed once) or borrowed (used without transfer of ownership). The same principle applies directly to cryptographic values. `Linear<Field>` means consumed exactly once. `Affine<Field>` means consumed at most once. The compiler enforces these constraints without runtime checks — every violation is a compile error.

## Design

### Core Types

```trident
type Nonce   = Linear<Field>;   // consumed exactly once
type Witness = Affine<Field>;   // consumed at most once
type Secret  = Linear<Field>;   // alias for nonce in secret-key contexts
```

`Linear<T>` requires the value to be used in exactly one subsequent expression. `Affine<T>` requires it to be used in at most one. Values that are neither linear nor affine are unrestricted (default `Field`).

### Usage Rules

```trident
fn use_nonce(n: Nonce) -> Commitment {
    commit(n)  // n is consumed here
    // After this call, n is gone — type system tracks consumption
}

// Compile error: nonce used twice
let n: Nonce = fresh_nonce();
let c1 = use_nonce(n);   // n consumed here
let c2 = use_nonce(n);   // ERROR: n already consumed
```

The type system tracks linearity through every expression. Branching over linear values requires that both branches consume the value exactly once (for `Linear`) or the non-taken branch proves the value is dropped (for `Affine`).

```trident
let n: Nonce = fresh_nonce();
if condition {
    use_nonce(n);    // n consumed in this branch
} else {
    drop_nonce(n);   // n must be explicitly dropped in the other branch
}
// After if/else: n is provably consumed in both paths
```

### Cryptographic Guarantees

The linear type system provides compile-time guarantees that no testing or auditing can match:

- **Nonce reuse prevention**: A `Nonce` value cannot appear in two commitments, two signatures, or two encryptions. Every nonce use is the sole use.
- **Witness confidentiality**: A `Witness` value cannot be returned in a public output, passed to a function that returns a public value, or stored in a location that is later revealed. The type system enforces the public/private boundary.
- **Double-spending prevention**: A token represented as `Linear<TokenCommitment>` cannot be consumed twice. The linear constraint is the on-chain ownership guarantee.
- **Secret key safety**: A `Secret` value cannot be copied, cannot be returned from a function, and cannot flow into any expression whose result is `Public<T>`.

### Interaction with ZK Types

Linear types compose with the `Private<T>` / `Public<T>` ZK type modifiers:

```trident
zk fn spend(
    token: Linear<Private<Field>>,  // private, consumed exactly once
    nullifier: &mut Public<Field>,  // public output
) {
    *nullifier = hash(token);
    // token is consumed — cannot be reused in another spend call
}
```

The compiler verifies that `token` flows into exactly one expression (`hash(token)`), that the result flows into a `Public<Field>` output, and that `token` itself never appears in any public context.

## Vision

In a world where [[cybergraph]] stores cryptographic state — nonces, witnesses, private keys used for [[bbg]] participation — linear types prevent the most dangerous class of protocol bugs at compile time. A nonce used twice creates a catastrophic vulnerability; a leaked witness breaks privacy. With linear types, these bugs cannot compile. Every deployed [[Atlas]] package carrying linear type annotations is provably free of cryptographic misuse.

This is not a linter. It is not a test suite. It is a mathematical guarantee embedded in the type system: any program that compiles with `Linear<Field>` nonces has, by construction, never reused a nonce. The [[zheng]] proof of the [[nox]] execution trace inherits this guarantee — the proof certifies not just that the computation ran correctly, but that cryptographic hygiene was maintained throughout.

For [[bbg]]'s participation model — where individual contributions are private and the aggregate is publicly verifiable — this is foundational. Every participant's focus contribution involves a witness that must not be reused. Linear types enforce this at the language level, before any proof is generated.

## Stack Integration

The privacy model of [[bbg]] relies on the call mechanism (Layer 2, pattern 16). Linear types enforce that each call witness is consumed exactly once — matching the Layer 2 semantic that each call atom is popped from the tape exactly once per `call_witness` pattern in [[nox]]. The type system and the VM semantics are in structural correspondence.

When a [[soft3]] `submit()` call packages a transaction involving private witnesses, the linear type checker has already verified that no witness appears twice in the submission. The [[zheng]] proof then provides the cryptographic certificate. Two layers of guarantee: compile-time (type system) and runtime (proof system).

Double-spending prevention becomes a type error rather than a protocol check. A token represented as `Linear<TokenCommitment>` cannot be passed to two different spend functions — the second use is a compile error. [[Atlas]] packages that handle token transfers with linear types are provably double-spend-free, and this property is visible in the package manifest at deploy time.

## Key Tradeoffs

**Ergonomics**: Linear types require explicit drops for values consumed in only one branch of a conditional. This is more verbose than conventional code. The tradeoff is that every cryptographic misuse becomes a type error — the developer is never silently wrong.

**Escape hatches**: Some legitimate patterns require breaking linearity temporarily. A value may need to be committed and also logged for debugging. The type system must support controlled escape with explicit annotation (`#[allow_copy]`), and these annotations should be auditable.

**Interaction with the proof system**: A `Linear<Field>` value consumed in the program corresponds to a field element that appears exactly once in the nox execution trace. This is a STARK-provable property — zheng's constraint system can verify linearity as part of the proof. The type-level check is a fast compile-time shortcut; the STARK provides the cryptographic guarantee. Linearity violations at the trace level would cause the zheng verifier to reject the proof.

**Performance**: Linear type checking is a flow analysis over the control flow graph. For programs with complex control flow (many branches, nested loops), the analysis may be expensive. In practice, cryptographic code tends to be structured (limited branching over secrets), so the analysis should terminate quickly.

## Implementation Sketch

Linear type checking integrates with the type checker as a flow analysis phase:

```rust
// typecheck/linear.rs
#[derive(Clone, Debug)]
enum Linearity {
    Unrestricted,
    Affine { consumed: bool },
    Linear { consumed: bool },
}

struct LinearEnv {
    vars: HashMap<VarId, Linearity>,
}

impl LinearEnv {
    fn consume(&mut self, var: VarId) -> Result<(), LinearError> {
        match self.vars.get_mut(&var) {
            Some(Linearity::Linear { consumed: true }) =>
                Err(LinearError::AlreadyConsumed(var)),
            Some(Linearity::Affine { consumed: true }) =>
                Err(LinearError::AlreadyConsumed(var)),
            Some(lin) => { *lin = set_consumed(lin); Ok(()) }
            None => panic!("Variable not in scope"),
        }
    }

    fn merge_branches(a: &LinearEnv, b: &LinearEnv) -> Result<LinearEnv, LinearError> {
        // Both branches must leave linear variables in the same consumed state
        // Affine variables may be consumed in one branch but not the other
    }
}
```

The linear check runs after type inference and before TIR lowering. Any linearity violation produces a compile error with the source location of the first and second use.
