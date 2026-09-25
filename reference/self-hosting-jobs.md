# Native compiler jobs and artifacts — SH0.3

Version 1. This specifies the structured bootstrap transport for
[self-hosting on soft3](self-hosting.md). Source-language compiler and Joy CLI
support are implementation gates, not implied by this specification. Data
wrappers use [SH0.2](self-hosting-data.md); complete noun files use nox's
[`NOXDAG01`](../../nox/specs/artifact.md) container. Host paths and JSON objects
never define native program/source identities.

## Records and profiles

Record notation `TAG(a,b,c)` means `[TAG [a [b [c 0]]]]`: exact arity, in field
order, with final atom zero. Tags are canonical Field atoms given below in hex;
their bytes are labels for humans, while the noun stores the numeric value.
Strings and raw sources are SH0.2 Bytes. Variable lists are SH0.2 Seq. Digests
are native `[[h0 h1] [h2 h3]]` with four canonical limbs in native order.
No schema field may be silently omitted, defaulted, reordered or ignored.

| Record | Tag | Fields in order |
|---|---|---|
| Job | `0x4a4f4231` (JOB1) | expected_compiler, package, entry_module, entry_function, options, limits |
| Package | `0x504b4731` (PKG1) | modules |
| Module | `0x4d4f4431` (MOD1) | logical_path, origin_name, origin_version, source |
| Options | `0x4f505431` (OPT1) | target, input_profile, output_profile, optimization, cfg_flags |
| Limits | `0x4c494d31` (LIM1) | source_bytes, modules, diagnostics, sequence_length, validation_visits, artifact_bytes, artifact_nodes, artifact_depth, reductions, arena_nodes, evaluator_frames |
| Result | `0x52455331` (RES1) | job_identity, status, payload |
| Program artifact | `0x41525431` (ART1) | machine, input_profile, output_profile, formula |
| Diagnostic | `0x44494131` (DIA1) | code, module_index, start_byte, end_byte, message |

`expected_compiler` is the full identity of the compiler **ART1 root** loaded by
Joy. The runner checks this before execution. The job identity is the identity
of the complete JOB1 noun. A result must bind that exact identity. Producer/job
identity belongs to RES1 or the execution receipt; it must not be embedded in
ART1. This keeps identical source/options able to produce identical executable
artifacts across C1/C2/C3 generations. Bootstrap compares complete canonical
ART1 bytes, not the provenance-bearing RES1 envelopes.

On success, expected compiler/job identities, execution counters and LIM1
admission caps must not affect ART1 or its formula. Limits may reject or fail
compilation; any code-affecting policy must instead be explicit in OPT1.

Machine/target value is Field `0` = native nox<Goldilocks,Hemera,word32>. Unknown
machine/profile values reject. Initially supported entry profiles are:

| Value | Name | Subject/result semantics |
|---|---|---|
| 0 | raw-noun-v1 | One complete Noun, transported without list adaptation or flattening |
| 1 | compiler-job-v1 | Input must satisfy JOB1; output must satisfy RES1 |

For ART1, both profile fields are explicit. Version 1 permits only the pairs
input0/output0 and input1/output1; mixed pairs reject because RES1 validation
requires JOB1 context. A compiler
artifact has input1/output1; a scalar arithmetic demo has input0/output0 and
returns an atom. Old flat-word typed entries remain their existing bundle
profile. A hand-authored raw formula may be wrapped with profile0 explicitly;
it must never acquire profile1 by file-extension guessing.

Options set target0, declared generated-program input/output profiles,
optimization0 (no optional optimization passes) and a sorted, unique Seq of
ASCII cfg identifiers. The job's flags explicitly include `release` for release
builds; no hidden host/env/default flags apply. Any other optimization level
must be rejected until specified. Literal constant folding required for language
semantics remains allowed. Intrinsic semantics come from the compiler/machine
profile; arbitrary source declarations cannot replace them.

## Source packages and resolution

Modules are sorted strictly by logical path's ASCII bytes, with no duplicates.
Paths match `[A-Za-z_][A-Za-z0-9_]*(\.[A-Za-z_][A-Za-z0-9_]*)*` and are at most
255 bytes. Entry module and function are each one identifier without dots,
at most255 bytes, matching the existing `program IDENT` declaration grammar.
Origin names/versions are nonempty printable ASCII byte strings, at most255
bytes each. They identify the chosen lock entry; they do not initiate fetching
or constrain versions by an implicit semver algorithm.

The module record identity binds path, origin/version and exact source bytes.
The package root binds all module records; these are the versioned dependency
identities. The host supplies the complete resolved source set, including needed
`std.*` and `vm.*` declarations. Two versions defining the same logical path are
an error, not search-path precedence. Changing even one source byte changes the
module and package identities. Absolute paths, timestamps and checkout order
never enter the package.

The guest resolves each `use` against this exact module table, checks declared
module names, rejects missing modules/cycles and links in deterministic dependency
order. To match the seed, visit reachable roots in lexical logical-path order,
walk each root's dependencies in first-use source order, and append a module
after its dependencies. Traversal deduplicates modules; binding retains every
original `use` occurrence, including repetitions. This order is distinct from
choosing the lexically smallest currently dependency-free module.
The selected entry module must be present and declare
the matching program name; dependencies declare modules. Imported aliases and
function/private-name rules retain the language contract. No fallback to host
files, embedded host stdlib, registry, witness callback or network is permitted
during the native compilation. Structurally valid packages may contain unused
modules; they remain identity-bound. Only the transitive entry closure is compiled;
unreachable source bytes cannot supply missing imports or override a module.

Header paths consist of identifier tokens separated by dots; each consumed dot
requires another identifier. Whitespace and line comments between tokens remain
part of original byte spans but never become logical-path bytes. Program names
contain one identifier; module owners and use paths may be dotted. The header
reader retains the full use span and the path span separately, then stops at the
first non-use token for the body parser. A lexical error encountered while reading
that boundary still fails the header. A wrong program/module kind or declared owner reports
code3 at that declaration; malformed path syntax reports code2 at the unexpected
token (including an empty EOF span), invalid lexical tokens report code1, and an
overlong normalized path reports code7. Complete body syntax and semantics remain
the responsibility of the later compiler stages.

The first guest graph component shares a 4096-byte budget across all reached
sources, including the entry. Each source is opened and UTF-8-checked once;
repeated uses retain separate spans and still charge package lookup work. Reached
module and use counts and traversal frames are each bounded by the smaller of
the job sequence limit and4096, checked before append. Original package indices
and compact reached-module IDs are distinct. A missing import or cycle points
to the caller's full use span; a reached encoding/name error points into the
reached source. A source-capacity error identifies the target package index with
an empty span. Unused sources receive no guest header or UTF-8 inspection.
This component currently resolves canonical import spellings exactly. Applying
the seed's documented legacy remaps before lookup remains part of complete guest
resolution acceptance; this component alone does not close that gate.

The host may collect and sort exact files into a package; it may not run compiler
stages to finish the emitted program. Module graph discovery/name checking and
semantic diagnostics in an accepted self-build must happen inside nox. Package
validation can check byte/schema/order/limits without parsing source.

Source byte strings are exact, including CR/LF, zero and invalid UTF-8. For modules
in the reachable entry closure, the guest reports invalid UTF-8 as a lexical
diagnostic before parsing, with byte positions;
it never normalizes source text. Identifier policy is the language's ASCII
grammar. Unused source bytes receive structural/size validation but are not
lexed, header-checked or semantically checked. Every span is `[start_byte,end_byte)`, bounded by the original module's
byte length. Empty spans at EOF are valid.

The guest's qualified-name reader retains the original full expression span and
the final member token separately. It normalizes only the module prefix into
ASCII dotted bytes, skipping source whitespace/comments through the lexer.
The package's 255-byte logical-name ceiling applies to that prefix; member
identifiers retain the source-language length bound. An identifier is required
after every dot. Lexical failures retain code 1, incomplete paths code 2 and
prefix-capacity failures code 7, all at the original offending token.
Imported constant lookup compares a member against its defining source, keeps
the calling expression's span, and retains literal provenance independently of
the canonical Field value. Direct-use order determines each exported binding;
only final public declarations enter an importing scope.

The initial C1 import slice checks dependency modules containing imports and
Field/U32 constant declarations. Other dependency declarations report unsupported
construct (code 6). Every declaration is checked, including private declarations
and declarations replaced by a later binding. After checking a module, only its
final public bindings are published to its direct importers. A published alias
keeps its own defining name and the terminal literal's original owner/span.
Entry expressions and constant initializers can reference full or short direct
module aliases. Lexical variables shadow a module root; a local constant with
that name does not hide the module alias. Imported calls, constructors and types
remain unsupported in this slice. Symbolic array extents and loop bounds retain
their existing unsupported diagnostics; ordinary constant expressions use the
normalized runtime value, including checked runtime indexing.

## Result and failures

Status Field0 = success, with exactly one ART1 payload. Field1 = compile error,
with a nonempty Seq of DIA1 records and no executable artifact. Other values
reject. Diagnostics are sorted by `(module_index,start_byte,end_byte,code,message
bytes)`, retaining duplicate occurrences if produced. Messages are valid UTF-8
Bytes and are explanatory; stable U32 error codes define machine meaning.

| Code | Meaning |
|---|---|
| 1 | Invalid source encoding/token |
| 2 | Syntax error |
| 3 | Missing/mismatched module or entry |
| 4 | Import cycle |
| 5 | Name/type/semantic error |
| 6 | Valid syntax outside this compiler's implemented subset |
| 7 | Compiler data/work capacity exceeded before runtime failure |
| 8 | Diagnostic limit exceeded |

Module indices refer to sorted package order. A package/entry-level diagnostic
uses the selected entry module and span[0,0); an absent entry is a job-admission
failure because no valid module index exists. Diagnostic count is bounded; upon
overflow preserve at most limit-1 ordinary diagnostics and append code8 at entry
span[0,0), then sort. The diagnostics limit must therefore be at least1.

Malformed container/schema, wrong compiler binding, unsupported machine/profile,
invalid limits and runtime faults are **job failures** outside RES1. A trap,
timeout or arena/reduction exhaustion cannot be relabelled a successful compile
or a guest-produced diagnostic. The runner records its distinct failure and
publishes no output artifact. Successful result validation checks job identity,
ART1 target/profiles against options, complete formula noun and all configured
limits before atomic publication. Formula execution/proving is a separate step;
schema acceptance is not a theorem that all inputs terminate or return typed data.

## Bounds and transport

All LIM1 fields are U32 except reductions, which is a canonical positive Field
integer. All fields must be positive, subject to the worker's supported caps;
the worker rejects requests it cannot honor before execution. `source_bytes`
bounds the sum of all module sources, not each file alone. `sequence_length`
also bounds library tables. `validation_visits` is a shared validation allowance
within each admission boundary (input JOB1, output RES1, or a guest validation
pass), never reset per attacker-supplied record. Distinct boundaries have separate
allowances; actual guest validation also consumes the execution reduction budget.
Fixed JOB1/LIM1 reads before extracting the request count toward that input
allowance. `artifact_bytes/nodes/depth` apply to
each complete container; nested logical validators also consume shared work
limits. Names/versions, records, source and diagnostics count toward their enclosing
container limits. Job limits must fit the worker's independently supplied admission
limits; parsing the untrusted limits record never raises those caps.

The guest package reader retains the admitted module table, its indexed-read
cost, explicit options, source limit and remaining validation allowance in the
entry view. Subsequent module lookup and source opening consume that same
allowance. Lookup compares complete logical paths and returns an explicit
found flag plus the original package index; source/AST sentinels never stand
for package indices. Opening a source checks the caller's remaining source
capacity before validating its Bytes payload. Reachability, once-only source
accounting, UTF-8 and language semantics belong to the guest module resolver.

Reduction, allocated arena-node and pending evaluator-frame limits are distinct.
The worker separately bounds trace retention, total memory and wall time; those
host admission limits cannot silently change generated code. The actual arena
load ceiling, loop/function policy, counted execution and trace retention are
fixed by SH0.4. Requested limits do not prove runtime capability. Initial runtime
implementations may support a finite set of capacities and reject larger jobs.
Proof-profile bounds are separately admitted; run support does not imply proof
support. Native compiler jobs have no secrets/state/network callbacks.

Every file is a complete NOXDAG01 container with one root: ART1 for programs,
JOB1 for compiler input, RES1 for compiler output, or an explicitly profile0 raw
input/output noun. A file's declared root must resolve to its entire canonical
DAG. Truncated payloads, missing/forward references, duplicate or unreachable nodes,
noncanonical words/order and resource excess reject. Artifacts must never be
converted through flat output leaves or recursively expanded bracket text.

Joy owns bounded file reads and temporary-file plus atomic-rename publication.
The host can serialize the formula noun already produced by the guest. It must
not translate AST/TIR, repair formula topology, append code, resolve imports or
otherwise perform language work after C1 was built. The run receipt records
compiler ART1, JOB1 and RES1 identities, extracted ART1 identity, source/package
identity, limits, consumed reductions, allocated nodes and trace mode.

Golden vectors and hostile-container tests belong to the implementation receipts.
SH0 closes only after runtime/control-flow policy and all owner interfaces are
reviewed; SH1/SH2 require executed structured transport and actual guest compilation.
