# Next SH1 delivery: reusable native control flow

Design checkpoint 2026-09-23, following the native Noun/raw ART1 slice.
Implemented on `feat/0.4-native-control` on 2026-09-24.
This design is preserved; the execution receipt records acceptance separately.
The [runtime contract](../../reference/self-hosting-runtime.md) is normative.

## Delivery boundary

Introduce reusable lowering through the raw ART1 API first. Keep the flat
bundle/proof lowering until its changed formulas, costs and proof coverage
receive their own migration. The raw profile continues to reject host services.
Do not claim native Zheng support from run-only acceptance.

Use a closed immutable balanced code table and balanced environments. A private
subject is `[code_table [state_root_or_zero environment]]`. Environment slots
hold entire subtrees, including Noun; assign stable slots for parameters in
source order, then lexical locals and generated temporaries. Shadowing gets a
new slot. Assignment rebuilds the path. Leaving scope removes names; unrelated
outer updates remain. Public input, array, tuple and struct encodings retain
existing contracts. Internal environment layout does not change their shape.

Plan reachable functions/specializations and generated loops before emission.
Sort by lexical module, function and concrete size arguments; generated entries
add source position and a deterministic local index. Assign table leaves before
lowering bodies. Quote the complete table once at the entry. Bodies refer to
other entries through the current subject; the serialized noun remains acyclic.
Use a dedicated source call-graph cycle check, independent of generated loop
cycles. Existing inline call-stack detection cannot enforce this after emission
stops descending into callee bodies.

Calls build a fresh callee environment and apply its table entry. Parameter
leaves appear in source order, so cons evaluates each argument left to right
exactly once. Alternative marshalling must materialize those arguments first.
Return only the function value; callee locals never escape into the caller.
Defining-module aliases, constants and struct layouts remain lexical.

## Blocks and loops

A statement/block yields a private flow result, Continue(environment) or
Return(value). Sequential composition executes its next block only on Continue;
Return propagates. Function boundaries unwrap the returned value. This replaces
the current CPS lowering which copies subsequent statements into returning arms.

A loop has candidate/remaining slots and one table body. Empty remaining count
finishes before evaluating a guard. Constant-end loops execute every candidate;
dynamic-end loops evaluate the end in the current environment for every admitted
candidate, even after a false guard. A false guard skips only that body. Return
propagates immediately. Decrement remaining before deciding whether another
index increment is needed, preserving u32::MAX as a valid last candidate.
Nested loops have independent slots; body-local names do not escape.

Two existing accepted cases require explicit compatibility work:

- `a[i]` works today inside an unrolled loop because i is constant. Reusable
  bodies need checked dynamic read/write before replacing that lowering. Keep
  public cons-list array layout initially; generated cursor/zipper helpers can
  access and reconstruct it without copying the source loop body.
- `for i in 0..3 { for j in 0..i bounded B { ... } }` and `for j in i..3` are finite today
  because outer unrolling specializes i. Preserve these via immutable loop-index
  range metadata. General dynamic starts remain outside this initial contract;
  do not accidentally accept arbitrary mutable bounds as formerly constant.

## Implementation slices

1. Deterministic code-table discovery, source-cycle rejection and balanced
   frame paths, with a small callable raw entry.
2. Reusable calls, once-only argument evaluation and lexical context tests.
3. Flow-return lowering, checked dynamic indexing and loop candidate machinery.
4. Differential/rejection corpus through Joy and explicit resource limits.
5. Native Seq/Bytes source libraries, then compiler JOB1/RES1 admission and SH2.

Suggested modules under `src/ir/tree/lower/nox/native/`: plan, layout, calls,
flow, loops, index. Reuse existing module identity/state/path code where it
preserves the representation; do not carry the shifting cons-list Scope into
balanced environments. Keep each new source file within the repository limit.

## Required observations

- Diamond graph size follows definitions/call sites rather than transitive
  expansion; one body per concrete specialization and deterministic table order.
- Calls evaluate arguments once left to right; first failure prevents later
  arguments. Local return completes that call. Source recursion still rejects.
- Same compact compiled loop executes 0/1/4097/5000 candidates. Increasing a
  bound changes constants, not a proportional number of body copies.
- Dynamic guard skip followed by later guard, changing end values, nested loops,
  early returns, outer mutation, shadowing, u32::MAX and bounded0 retain behavior.
- Existing loop-index array access and outer-index-dependent nested ranges work.
- More than 58 live bindings work through balanced environments; raw Noun
  arguments/results retain their complete topology.
- Node/frame/budget exhaustion fails explicitly, never as a successful shortened
  computation. Record actual cost, node counts and peak frames separately.
