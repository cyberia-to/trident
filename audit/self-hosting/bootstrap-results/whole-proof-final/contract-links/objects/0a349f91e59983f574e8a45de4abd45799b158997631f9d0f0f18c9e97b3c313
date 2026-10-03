# Public structured native execution certificate

Status: 0.4 implementation contract. Actual compiler workload and whole-self-build
acceptance require their separate retained SH7/SH8 receipts.
Profile `joy-nox-disclosed-compiler-v1` carries a full public witness for the
successful pure nox tags0..15 relation. Zheng's disclosed noun memory and bounded
semantic stream own primitive equations, dynamic continuations, chosen branches,
ordered children, exact charge and logical invocation depth. Verification never
calls the nox evaluator or compiler. Work and certificate size are linear in the
actual disclosed records; succinctness and zero knowledge are separate profiles.

## Admission and public context

The caller supplies complete expected PROGRAM and INPUT NOXDAG01 files. Both
proving and verification use the existing ART1 and compiler JOB1 admission rules.
Raw ART1(0,0,0,formula) takes INPUT unchanged. Compiler ART1(0,1,1,formula) takes
the complete admitted JOB1 unchanged, including sources, dependencies, options,
compiler identity and requested limits. No host compiler stage supplies output.

The transport's independently computed context is Hemera of
`joy/nox/disclosed-compiler/v1\0`, followed by the complete program particle,
formula particle, input particle (four canonical little-endian u64 limbs each),
one profile byte0 or1, initial budget u64 and admitted logical frame bound u32.
The machine is fixed to nox0 by ART1 admission and this profile domain. Caller
resource ceilings are enforced independently rather than accepted from proof
metadata. Compiler budget/frame values come from the admitted LIM1 request.

## Semantic payload

Payload bytes use [bounded transport](compiler-certificate-transport.md).
Records contain a one-byte tag and these unsigned little-endian fields:

| Tag | Record | Fields |
|---:|---|---|
| 0 | Atom | value u64 |
| 1 | Pair | left u32, right u32 |
| 2 | Reset | epoch u64, initial noun count u32 |
| 3 | Enter | object u32, formula u32 |
| 4 | Finish | result u32, cache slot u32; MAX means no storage |
| 5 | Reuse | slot u32, generation u64 |
| 6 | Terminal | result particle, exact charge u64, NOXDAG01 byte count u32, complete result bytes |

The first record is Reset epoch0. Further reset epochs increase by exactly one
with checked arithmetic. Reset discards the entire current noun table. Exactly
the declared positive noun count must follow before any semantic record or
another reset; the count must fit the configured resident noun allowance. Later
fresh nouns append to that same table. Every atom is canonical and every pair
names prior indices. The verifier independently derives all Hemera particles
and complete Cost metadata. Reset imports no trusted metadata and preserves
the semantic activation stack, cache generations and accumulated metrics.

Enter, Finish and Reuse invoke the corresponding Zheng semantic stream methods.
Every noun index addresses the current epoch. Activations and cache entries retain
checked particles and header facts, so none keep such indices across reset.
Cache slots are bounded and only a checked Finish populates them. Stale handles,
wrong invocation keys, wrong results or continuation order reject. Reuse charges
the complete stored evaluation metrics each time.

One Terminal is required after exactly one completed root derivation. Its root
must match context INPUT/formula, result particle and exact charge. The admitted
budget must cover the charge and frame bound must cover the derived peak. The
complete result bytes are independently decoded and their particle must equal
the relation's terminal result. Compiler results undergo existing complete RES1
admission, binding JOB1 identity, status, diagnostics and any extracted ART1.
Success/program and compile-error/diagnostics are both successful VM outcomes.
After terminal payload, immediate transport terminal and underlying EOF are
required. Prefixes and trailing records cannot be accepted.

## Producer and resource claims

The producer observes successful nox compacting execution through observer v2.
With no explicit compaction policy, collection work is zero and an execution
which needs collection fails. Opting into resident storage and collection work
uses the same explicit host policy as structured execution.
Initial and post-GC resident snapshots become independently checked noun epochs;
fresh noun exports precede semantic records. Runtime-provided identity and Cost
metadata are cross-checked against admitted definitions. Observed transition
sequence, declared snapshot counts and fresh-node counts are also checked.

A cache hit emits Reuse at the invocation's observed Enter. While nox executes
that invocation, the producer suppresses only descendant semantic records. It
continues noun/snapshot processing and compares the matching runtime Return's
particle and consumed budget with the previously verified summary. The proof
may be published only after successful runtime return, complete observer capture,
result admission, semantic terminal verification and finished transport output.

Producer construction reserves a bounded invocation-budget stack. Its current
epoch dictionary maps full particles to checked noun indices; its cache-key
dictionary is bounded by the configured number of summary slots. Begin must be
version2 and match the admitted root, budget and frame allowance. It declares
the initial snapshot count. Each Node is independently admitted before its
claimed particle and Cost are compared and its record is written. A reset must
name the next native transition sequence, follow completed snapshot/fresh-node
groups, and advance the proof epoch. Snapshot nodes and fresh allocations have
separate counters. Each Transition consumes the expected sequence and exact
fresh-node count.

Every observed Enter/Return updates the bounded invocation-budget stack, including
suppressed descendants. Ordinary Finish checks its derived result and exact cost
against the matching runtime Return before publication. Cache slots are replaced
in deterministic round-robin order; the old key is removed only if its dictionary
handle still names that occupant. Reuse validates slot generation and key before
copying the opaque summary for its suppression interval. Exactly one Completed
event must match the root result, remaining budget and transition count. Producer
completion requires an empty invocation stack, no suppressed invocation and no
unfinished snapshot/fresh-node group. Duplicate Begin/Completed, events after
completion, malformed state or sink failure leave only an unpublished prefix.

Independent caps cover encoded bytes, decoded bytes, transport frames, semantic
payload records, noun slots, summary slots, expanded steps, charge and invocation
depth. The surrounding execution/verification worker enforces a cooperative
deadline and admitted schema/transport caps. Record count includes nouns and
resets, so arbitrary memory churn consumes the allowance. Prover lookup maps
are bounded by resident nouns and cache slots; their standard BTreeMap node
allocations follow Rust's global allocation failure policy. Runtime failure or
interrupted capture cannot publish a completed certificate.

The proved resource quantities are exact semantic charge, expanded invocation
count/steps and logical invocation peak. Physical allocation, resident storage,
GC count/work, elapsed time and RSS remain host observations. Admission checks
requested physical ceilings, but this logical relation supplies no proof of
every physical LIM1 or host limit. Declared disclosure covers source, input,
intermediate nouns and output. Existing private native proof routes retain their
own contract and never silently select this disclosed format.

## Production commands

`joy prove-artifact PROGRAM --input INPUT --output CERTIFICATE [--force] [LIMITS]`
stages and atomically publishes one complete certificate. `joy verify-artifact
PROGRAM --input INPUT --proof CERTIFICATE [--output OUTPUT] [--emit result|program]
[--force] [LIMITS]` verifies against the expected files and optionally publishes
the complete result or extracted compiler ART1. Both commands accept the existing
structured runtime and compiler admission limits. Proof noun slots equal the
selected host resident allowance. Additional independent positive caps are:

| Flag | Default | Hard maximum |
|---|---:|---:|
| --proof-bytes | 67108864 | 137438953472 |
| --proof-decoded-bytes | 268435456 | 1099511627776 |
| --proof-records | 4000000 | 100000000000 |
| --proof-steps | 2000000 | 40000000000 |
| --proof-cache-slots | 65536 | 262144 |

Each data frame contains at least one decoded byte; decoded byte allowance plus
one also bounds transport frame count. The encoded-byte cap includes all frame
overhead. Native observer event/work/byte ceilings are checked arithmetic derived
from proof step and record caps; actual observed transitions additionally obey
the proof step cap during cache suppression. The verifier separately charges
all noun/reset/semantic/terminal records before their semantic work.

Existing destinations survive any admission, computation, proof or publication
failure. Production input/proof files use bounded regular-file admission and
reject links/streams. `--force` replaces only a complete successful output.
`--emit program` requires a successful compiler RES1; compile errors retain
diagnostics and cannot publish a program. A successful prove reports
`joy/artifact-proof/v1`; verification reports `joy/artifact-verification/v1`.
Reports distinguish verified semantic quantities from optional observed producer
counters. Fresh verification returns no claimed producer observations. Its
program/input are explicit external expectations and it performs no execution.

The worker deadline covers admitted decoding, bounded semantic operations,
capture and result validation cooperatively. Initial file reads, underlying
blocking I/O and final durable filesystem synchronization are bounded by byte
admission or the filesystem, rather than a preemptive timeout.
