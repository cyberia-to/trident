# Native compiler: typed branches and scoped return flow

Source: Trident `0efff6c1658c6d89218e3c009caeea96457575ab`, Joy `a3dd4c5c7c2f870f6632deace5b137a141796173`,
Trisha `908d5e22a669e4aa0b6c02b5e9134e67c1a25675`, nox `5271961a72a1f1e9922df36e3e4e3c65d1acff81`.
[The pinned validation receipt](sh3-native-control-validation.json) records local
development inputs and commands. This closes the Bool/control slice of SH3:
Field/Bool locals, equality, scoped if/else/else-if, expression statements and
explicit return compile inside nox into separately executable ART1 programs.

The parser checks every arm and restores its parent's visible bindings after
closing a block. Each declaration retains a unique runtime slot. Outer mutable
writes survive the branch; inner names and shadowing stay local. Owned statement
lists and postorder blocks preserve ordering through nested and synthetic blocks.
A separate terminal-context pass returns only function-terminal tails. Earlier
branch tails are evaluated and discarded. Generated Continue/Return records
execute each continuation once and bypass it on return.

Field-condition documentation now agrees with the existing seed and formal-audit
contract: nox canonical zero selects then; Triton nonzero selects then. Bool
conditions retain their logical meaning. This corrects stale prose and preserves
the established target behavior. [The subset contract](../../reference/self-hosting.md#native-compiler-subset-contract)
defines the accepted language, error codes and limits.

## Execution acceptance

All three installed owners were rebuilt after the implementation commit before:

```sh
python3 audit/self-hosting/run-native-compiler.py \
  --joy ../install/bin/joy --output /tmp/native-control-cli.json
```

[The installed CLI receipt](native-control-cli.json) records 171 commands
and 60 observations. It builds C1 once, then creates the fresh source
corpus. Joy packs exact files, executes all compiler stages inside nox, publishes
the returned ART1 and independently runs that artifact. Successful prior arithmetic
and Field-local cases retain their accepted artifact identities; arithmetic also
matches independently constructed exact formulas. Type/scope/return errors,
unselected invalid arms and malformed control return diagnostics. Failed compilation
and exhausted execution allowances preserve the previous destination file.

C1 particle: `390b65f6a8d666615e5182ddce72e9b929a5ab9ade288869349c3a1ab301df29`.
The following measurements come from the installed command and source revision
above; allocated nodes count lifetime allocation rather than live memory.

| Case | Generated result | Charged reductions | Allocated nodes | Peak frames |
|---|---:|---:|---:|---:|
| precedence | 14 | 850440 | 51106 | 473 |
| typed-local | 7 | 952519 | 48559 | 565 |
| bool-precedence | 11 | 1796256 | 77896 | 653 |
| early-return | 7 | 1825162 | 73047 | 653 |
| else-if | 7 | 2440640 | 91527 | 686 |
| branch-scope | 7 | 2099843 | 80847 | 653 |
| body-chunks | 9 | 3693956 | 134737 | 710 |

Compiler cost increased against the [locals receipt](native-compiler-locals.md)
from Trident `6edc4c198d41e20f2dc246666b932050187be832`, measured with the same
installed runner command on that revision. Arithmetic uses 850440 rather than
819755 reductions and 51106 rather than 41676 allocated nodes. The nine-assignment
case uses 3693956 rather than 3448420 reductions and 134737 rather than 111554 nodes.
The larger compiler and typed block representation add overhead; SH4 must address
compiler-scale allocation. Generated programs retain their prior identities.

The Rust source/JOB suite additionally compares independent expectations and
Rust-seed execution on identical programs, including constant-condition return
coverage, nested intermediate Bool tails, early returns before later code,
outer mutation versus local shadowing, and crossing parser/emitter chunk boundaries.
All 954 Trident workspace tests, 120 Joy workspace tests and 380 Trisha CPU tests
passed, with four existing Trisha tests ignored and no Rust warnings. Trisha
verified 133 fixtures and 43 independent manual baselines; result/cycle rows match
the preceding locals delivery. The validation receipt pins commands and revisions.

## Boundaries and remaining work

Full source/JOB tests exercise requested nesting and independent block-table caps;
`if true {} else {} 7` isolates root-block append capacity from expression and
nesting capacity. Larger successful caps preserve identical artifact bytes.
Nested source programs cross seven/eight/nine conditional levels. The actual
parser enter-operation's hard nesting boundary is tested separately with prepared
continuation frames; it does not claim full source/JOB execution at that ceiling.

The actual control generator has an independent DAG-depth probe covering branch,
frame edit, discarded expression and return continuation. Exact RES1 depth passes;
one less yields diagnostic7. Full JOB admission also bounds C1 depth, so this is
explicitly a generator component test, separate from the complete CLI corpus.

The installed source-ceiling and larger assignment workloads still exhaust the
configured execution allowance and preserve the old program. SH4 remains open.
The formal audit covers 22 files including the entry, all UNKNOWN;
these tests establish execution evidence without claiming a compiler proof.
Next are typed reusable functions and calls, then the remaining compiler language
and full-project resource scale. C2/C3 self-build, six-platform release acceptance
and native Zheng proofs remain open. Noun's roadmap temperature remains 128K.
