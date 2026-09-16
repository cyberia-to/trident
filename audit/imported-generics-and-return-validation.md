# Imported generics and return validation

The canonical contracts are in [language.md](../reference/language.md), sections
Size-Generic Functions, Constants, and Return. This change fixes executable
source semantics; it does not extend the nox match implementation or imply that
unsupported proof relations are complete.

## Reproduced defects and corrections

1. Public generic signatures were exported as ordinary signatures with unknown
   dimensions evaluated as zero. Imported explicit calls failed with “not
   generic” and shape mismatches. `ModuleExports.generic_functions` now carries
   unresolved size parameters/types separately, including the canonical owner
   identity and lexical signature context. Ordinary exported functions no longer
   contain fabricated zero-sized generic signatures.
2. Generic bodies were skipped without subsequent concrete body checking.
   Project preparation now resolves exact call sites, allocates deterministic
   collision-free concrete function names, and materializes each function in
   its defining module. It repeats type checking before lowering. Nested calls
   infer sizes from their actual concrete argument shapes; identical source
   spans in different concrete caller functions remain separate. Private
   helpers, cfg selection, nominal types, and lexical constants stay with the
   owner. Both nox and Trisha consume ordinary checked concrete functions.
3. Ordinary function explicit returns and terminal values were never checked
   against the declared return type. They now are, including missing fallthrough
   results, terminal if/match branch values, and fixed nonempty versus empty
   loops. Parameter/local shadowing cannot turn a runtime condition into a
   module-constant termination claim. A shared AST terminal-return normalization
   preserves expression spans and scopes for checking, formal analysis, and
   nox lowering. The nox lowering correction and its proof regressions are
   recorded by the parent agent separately.
4. Formal scalar analysis discarded terminal if branch values and could model
   their result as zero. It now consumes the same return normalization. A real
   Z3 regression rejects `ensures(result == 0)` for a terminal 7/9 conditional.
5. Bare module constants in shared TIR fell through to a stack duplicate.
   They now emit their constant value after local/spilled bindings take
   precedence. Constant references preserve declared Field/U32 types; U32
   initializers are range checked. Generic expression substitution preserves
   nominal module constants while substituting size parameters. Constants are
   registered before signatures so forward declarations are valid.

Dimension substitution rejects unresolved names and arithmetic overflow.
Literal out-of-bounds array reads, wrong generic arity/shapes, duplicate size
parameters, and recursive instantiation are rejected. Expansion has explicit
limits of 1,024 instances and 128 rounds. Recursion detection respects cfg.
Single-source compile, TIR, check and editor/project checking use the same
concrete preparation rather than bypassing body checking.

## Validation

- `tests/imported_generics.rs`: five tests cover actual nox execution in both
  profiles, nested imported entry shapes, lexical dimensions, malformed calls,
  invalid generic bodies, ordinary return errors, missing paths, nominal U32
  constants/range, and single-source API parity.
- Independent Trisha `rs/tests/imported_generics.rs`: five tests pass in both
  profiles. Cases include wide 20-word results, nested inferred sizes, two
  owner modules with the same function name, private helpers, cfg, module
  constants shadowed by size parameters and caller variables, and user names
  colliding with proposed specialization labels. Owner logs:
  `/tmp/trisha-imported-generics-review.log`.
- The owner also ran a genuine default-security Triton proof for input [3,5]
  and output 42, rejecting changed input, output, and program digest:
  `/tmp/trisha-imported-generics-proof.log`.
- The 12 real-Z3/formal/native tests pass with the terminal-if regression:
  `/tmp/generic-return-targeted.log`.
- The first complete workspace neural run passed **882 tests**, zero failures,
  zero ignored tests and zero Rust warnings across 19 test reports:
  `/tmp/generic-return-workspace-neural-final.log`. The final repeat after forward constants and canonical Field-condition
  handling passed **883 tests**, zero failures/ignored/Rust warnings, across
  19 reports: `/tmp/generic-return-workspace-neural-final3.log`.

The full library check initially exposed 19 module failures caused by the old
checker treating terminal branch values as unit. Once terminal values were
modeled correctly, only two source files required actual repairs:
`std/compiler/codegen.tri` and `std/compiler/typecheck.tri` had unit helpers
leaving Field-returning calls as their terminal values. The typed owner made
those discards explicit; the checker was not weakened to accept them.

Historical release archives and proof receipts are not represented as rebuilt
by these tests. Current release artifacts and the full proof corpus remain the
parent agent's coordinated release checks.

Final module check: **48/48** core-library and production SDK modules passed
`trident check --target neptune` using current API-3 owner discovery. The current
inventory is `/tmp/generic-return-inventory/results.json`; its historical JSON
key `audit_neptune` records this run's check exit code, not a formal proof.

Canonical Field handling received an additional boundary regression: scalar
program literals are reduced modulo Goldilocks for formal evaluation and branch
termination inference; compile-time sizes are not reduced. Symbolic negation of
zero now yields canonical zero. Trisha's textual immediate normalization is an
owner correction tested separately; the earlier parser rejection must not be
counted as successful Field-literal execution.


Coordinated frozen-source repeat: `cargo test --release --workspace --features
neural --locked` exited 0 after the final shared-TIR intermediate-value fix and
nox canonical Field regression. **884 tests passed**, zero failed/ignored,
zero Rust warnings, 19 test reports, including the **49-test nox surface** suite.
Log: `/tmp/trident-final4-workspace-reviewed.log`. Only this report was edited
after the run; no implementation changes were made by this agent.
