# Native compiler control flow and runtime

SH0.4 contract for Trident0.4 on nox through Joy. This specifies implementation
requirements; [runtime baseline](../audit/self-hosting/runtime-baseline.json)
records the recursive interpreter's actual limits. Source-level native calls,
loops and the heap evaluator require SH1 acceptance before being advertised.

## Source execution

Ordinary functions compile to reusable nox formulas. Each reachable function
specialization and each generated loop body has one code-table entry. Order
entries by logical module path, function name, concrete generic arguments and,
for generated continuations, source byte position and deterministic local index.
Do not use hash-map iteration, host allocation IDs or nondeterministic naming.
Quotation and native pattern2 (compose) select/apply a runtime formula. Pattern16
is witness injection and must never implement ordinary calls or compiler work.

The table is an immutable balanced noun tree, padded with zero, carried in the
execution subject. A body refers to that table through its subject; the table
contains no cyclic data or host pointer. Calls keep the same table and create a
fresh callee environment. Use balanced environments where linear axes would
exceed the native axis width; checked paths compose multiple axis operations.
Native `Noun` values remain subtrees, including inside aggregates/environments.
No field-width calculation may pretend they are one serialized field word.

Evaluate native call arguments exactly once, in source order, against the
caller environment. Then rearrange the already-evaluated values into the callee
layout. Stop at the first failed argument. This corrects the old native
inline-call fold, which accidentally evaluated arguments in reverse order.
The shared language currently has no general operand-order guarantee; this
native call contract does not silently change foreign-target binary operators.
The existing event declaration-order/once-only contract remains in force.

User recursion and mutually recursive function cycles remain rejected. Calls
through generated bounded continuations are an implementation mechanism, not
an extension allowing unrestricted source recursion. Explicit return immediately
leaves the current function, including from nested branches/loops; statements
and arguments after that return have no effects. Mutable outer bindings survive
an iteration, loop/body-local bindings do not escape. Callee locals never leak.

### Native bounded loops

Preserve the existing native loop's observable behavior during its replacement:

- The start must initially be a compile-time canonical integer. For a constant
  end, execute candidates start through end-1; end<=start is empty. The old
  native backend ignores a `bounded` annotation when end is constant.
- For a dynamic end and bound B, consider exactly B candidates start+j. Evaluate
  end once per candidate in the current environment. Execute the body only if
  that candidate is smaller than end. A false guard skips that body; later
  candidates still evaluate their guards. Return exits immediately.
- Reject a candidate range that cannot be represented by U32 indices without
  wrap; index u32::MAX is valid and completion must not increment it again. B=0 performs no guard or body evaluation. An omitted dynamic bound is
  rejected. Limits on compilation/execution do not redefine B.
- The generated loop tracks candidate, remaining candidates and environment
  explicitly and reuses one body. It must not clone the body B times. Calls in
  the body similarly reuse their specialization. Nesting preserves scope.

These native rules differ from the existing TIR backend's evaluate-once loop;
any cross-target harmonization is a separate language change with differential
coverage. Do not claim the current backends already agree on effectful bounds.
Compiler loops use admitted work lengths and explicit capacity checks: reaching
B while required compiler work remains produces diagnostics, never a successful
partially compiled program. Source guards are not resource-admission checks.

## Sequential nox execution profile

The compiler worker executes pure L1 with no jets, no parallel forks and no host
witness/state provider. Dispatch of reached tag16/17 is an explicit unsupported
profile failure; such numbers inside quoted data are allowed. Dynamic compose
must check its reached formula too. Optional Cargo features must not change
this profile's execution strategy or cause hidden trace retention.

Replace Rust recursive evaluator calls with a bounded heap continuation stack.
One active invocation counts as one frame; root counts as one. Pending parent
rows remain until children return. This removes dependence on the old1001-frame
host-recursion cap but does not promise constant-memory tail calls. Before
pushing a frame, check `evaluator_frames`; allocation failure/frame excess is a
runner resource failure, with no successful output artifact. No implicit limit
increase/retry may turn a failed acceptance job into success.

Preserve the existing sequential interpreter's pattern costs and observable
postorder trace, including multirow blocks. Evaluate binary children left then
right; check binary operand types after both children finish, as today. Stop on
the first child execution failure. A branch evaluates only its selected arm.

Budget partitioning remains exact: with post-dispatch budget B and static child
bounds a,b whose saturating sum fits B, give each child its bound and refund
unused budget on successful join. Otherwise thread the remaining budget from
left to right. Unary children follow the same bound/refund rule. Bounds on
branches are upper bounds, not minimum required budgets. Selected branches may
succeed below the other arm's bound. Compose applies its computed formula with
the joined budget, without another static reservation. On child failure, retain
the recursive reducer's failure outcome; do not invent refunded gas.

NoTrace and VecTrace executions must agree on output identity, remaining budget
and allocation order. On successful pure L1 runs, charged reductions equal
initial minus remaining budget, and the current trace row count agrees. Failure
rows are not a gas counter: budget0 emits an uncharged halt row; inv(0) charges
its weighted cost without producing a complete success block. Outcome::Error
currently loses the remaining budget. Receipts report unavailable failed-run
cost as null, never zero or a fabricated trace-derived count. Existing partial
failure traces remain compatibility evidence, not successful proof witnesses.

## Arena, transport and host resources

[JOB1 LIM1](self-hosting-jobs.md) supplies positive admission ceilings. The worker
also has independent hard ceilings. Check both before running; a job cannot
raise host policy. Keep these resources separate:

| Resource | Enforcement owner and unit |
|---|---|
| reductions | nox dispatch cost, canonical field integer budget |
| arena_nodes | nox lifetime distinct nodes, including loaded code/input and intermediates |
| evaluator_frames | nox active invocations, root included |
| artifact bytes/nodes/depth | nox NOXDAG01 on each complete input/output container |
| validation visits | bounded Joy/guest admission at each declared boundary |
| host memory, stack, elapsed time | Joy worker admission/supervision, independent policy |
| retained trace | off in run-only; separately bounded for proof collection |

The arena is append-only for one run. Hash-cons reuse is free in node count;
new persistent updates retain their history. Tightening a node allowance cannot
invalidate existing nodes or be reversed. A failed compound allocation may
leave charged nodes; it cannot return a partial success. Existing static arrays
reserve memory for their full slot count even when the logical limit is small.
Measure physical arena size, codec workspace, frame storage and trace mode;
logical node count is not measured RSS. Worker cancellation/time supervision
must not leave a detached computation that later publishes an artifact.

NOXDAG01 preserves unique DAG nodes and complete topology. Never flatten output
or recursively print shared nouns into exponentially expanded bracket text.
Encode and validate successful output fully before atomic publication. Existing
files remain intact on execution, encoding or publication failure. The host may
serialize/import/export nouns and validate protocol records; it must not lex,
parse, typecheck or finish the guest compiler's generated program.

## Joy admission and first SH1 delivery

Add an explicit structured raw-artifact run API/command alongside the existing
flat ProgramBundle path. Accept machine0 ART1 profile(0,0), a raw NOXDAG01 input,
and explicit transport/reduction/node/frame ceilings. Return complete raw output
plus its identity and successful charged cost. Profile(1,1) is admitted only
when production JOB1/RES1 binding/admission is implemented; a hand-built fixture
is not a compiler. Run-only uses NoTrace. No automatic proof fallback.

First acceptance commands belong in the implementing owner's receipt. Fixtures
must cover topology distinction, shared deep DAG, runtime-generated formula,
input/output byte boundaries, arena exhaustion during execution, frame failure,
budget failure, unsupported services, failed atomic publication and no-overwrite.
The heap evaluator must run the same compact baseline loop for4097 iterations
with result4097 and61460 charged reductions, while the old reducer still fails.
Differential small-program tests compare every trace column and allocated node
for success, failures, selected branches and partitioned/dynamic budget paths.

## Zheng consequences and owner review

The initial structured worker is run-only. Preserving L1 trace rows does not
extend the production Zheng relation: it currently rejects dynamic continuations
and variable shapes. SH7 must bind complete program/input/output particles,
execution budget and supported control flow to the same verified execution.
New artifact wrappers or copied source hashes alone prove nothing about that
relation. SH8 needs native proofs of both complete self-builds. Triton-backed
JOYZK003 remains a distinct profile and cannot satisfy native proof independence.

Owners: Trident specifies source semantics/code layout; nox enforces reduction,
allocation and frame behavior; Joy enforces transport/admission/publication;
Zheng owns relation/proof coverage. SH0.5 records an engineering review of all
four boundaries and pins its inspected commits; it is not a claim of approval
from four maintainers or of implemented SH1/SH7 functionality.
