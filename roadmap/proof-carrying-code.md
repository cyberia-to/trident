---
status: draft
author: mastercyb
area: interop
planned: 64K
---

# Proof-Carrying Code Distribution

## Motivation

Software is distributed on trust today. A signed binary asserts that its author is who they claim to be — it says nothing about what the binary does. A recipient who cannot read machine code must trust the author's reputation. For security-critical code, this trust is the weakest link in the entire chain.

Proof-carrying code replaces identity trust with mathematical trust. The distributed artifact is not just a binary — it is a binary paired with a STARK proof that the binary was compiled correctly from a verified source program. The recipient verifies the proof without re-executing the program. They learn what the program computes, not who wrote it. The author's identity is irrelevant; the proof is sufficient.

## Design

### Distribution Format

```
my_library.tri  →  trident build  →  my_library.tasm
                                       my_library.stark_proof

# Combined in a bundle:
my_library.warrior = {
    tasm:       my_library.tasm,
    proof:      my_library.stark_proof,
    public_io:  { inputs: [...], outputs: [...] },
    meta:       { source_hash, compiler_version, field }
}
```

The `.warrior` bundle is self-contained. It includes the compiled TASM (executable on any Triton VM), the STARK proof (verifiable by any standard verifier), the public input/output specification (so the verifier knows what was proven), and metadata for reproducibility.

### What the Proof Proves

The STARK proof in the bundle proves that the TASM binary is the correct output of compiling the Trident source program. More precisely, when the Trident compiler is itself a Trident program (self-hosting), the proof is a proof of compilation:

```
Input:  Trident source code S
Output: TASM binary T
Proof:  STARK proof that "compiling S produces T using compiler C"
```

A recipient who trusts the Trident compiler (or trusts the STARK verifier that validated the compiler's correctness) can accept T as a correctly compiled version of S without running the compiler themselves.

For non-self-hosting phases (compiler written in Rust), the proof is weaker: it proves the TASM binary executes the program's semantics correctly (the execution proof), not that the compilation was correct. Still valuable — the recipient can verify the binary's behavior without executing it.

### Verification Without Re-Execution

The recipient workflow:

```
Receive: my_library.warrior
Step 1: Extract public_io and proof
Step 2: verify_stark(proof, public_io) → bool
Step 3: If true: accept the library's behavior as proven
Step 4: Use my_library.tasm as a dependency
```

Verification takes milliseconds (STARK verification is fast). Re-execution would take seconds to minutes for complex programs. For libraries distributed at scale, each recipient verifies rather than re-executes — the computational burden shifts from recipients to the single originator who generated the proof.

### Composable Proof Chains

When `my_library` depends on `other_library`, the bundle chain composes:

```
other_library.warrior  (TASM + proof)
       ↓ dependency
my_library.warrior     (TASM + proof + embedded proof of other_library usage)
       ↓
final_program.warrior  (TASM + proof of entire program + proof of all dependencies)
```

The final program's proof transitively covers all its dependencies. A recipient who verifies `final_program.warrior` learns that the entire dependency chain executed correctly — not just the top-level program.

### Trust Model

The trust model for proof-carrying code has two components:

1. **Trust in the STARK verifier**: The recipient must trust that the STARK verification algorithm is implemented correctly. This is a small, auditable piece of code — vastly simpler than trusting a full compiler + runtime.

2. **Trust in the Trident source**: The recipient must trust that the Trident source program, if they can read it, does what they expect. If they cannot read it, they trust the STARK proof that the TASM matches the source.

What the recipient does NOT need to trust: the author's identity, reputation, or signature. Mathematics replaces identity.

## Key Tradeoffs

**Source availability**: For the recipient to know what they are accepting, they should be able to read the Trident source. Distributing TASM without source is equivalent to distributing a signed binary — the proof is still valuable (behavioral correctness), but the recipient cannot independently verify the intent.

**Proof size**: STARK proofs are hundreds of kilobytes. Distributing proofs alongside binaries increases distribution size. For large programs with large proofs, this may be significant. Neural proof compression (a separate proposal) addresses this.

**Compiler version pinning**: The proof is valid only for a specific compiler version. A recipient using a different verifier version may reject a valid proof or accept an invalid one. The `meta.compiler_version` field in the bundle allows recipients to check compatibility.

**Dynamic behavior**: The STARK proof covers a specific execution with specific inputs. If the program's behavior is input-dependent, the proof only covers the specific inputs listed in `public_io`. For libraries that are called with many different inputs, the proof covers the compilation correctness (that this TASM corresponds to this source), not the execution for every possible input.

## Implementation Sketch

The bundle format and tooling:

```rust
// runtime/artifact.rs (already exists, extending)
#[derive(Serialize, Deserialize)]
pub struct ProgramBundle {
    pub tasm:       TasmBinary,
    pub stark_proof: Option<StarkProof>,   // None for unproven bundles
    pub public_io:  PublicIO,
    pub meta:       BundleMeta,
    pub dep_proofs: Vec<DependencyProof>,  // proofs of dependencies
}

#[derive(Serialize, Deserialize)]
pub struct BundleMeta {
    pub source_hash:       [u8; 32],
    pub compiler_version:  String,
    pub field:             FieldId,  // Goldilocks, BabyBear, etc.
    pub created_at:        u64,      // Unix timestamp
}

// cli/distribute.rs
pub fn create_distribution_bundle(
    source: &Path,
    output: &Path,
) -> Result<ProgramBundle> {
    let tasm = compile(source)?;
    let proof = prove(&tasm)?;
    let bundle = ProgramBundle {
        tasm,
        stark_proof: Some(proof),
        public_io: extract_public_io(&tasm),
        meta: BundleMeta::current(&source),
        dep_proofs: collect_dependency_proofs(source)?,
    };
    bundle.write_to(output)
}
```

The `trident distribute` command wraps this workflow and generates the `.warrior` bundle file. The `trident verify-bundle` command verifies a received bundle without executing it.
