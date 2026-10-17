---
title: execution model
tags: cyber, soft3, spec, foundation, architecture, warrior, worker
crystal-type: spec
crystal-domain: cyber
status: normative
version: 1
alias: soft3 execution model, execution architecture, warrior model, worker model, warriors workers and networks
---

# execution model

One warrior supports a VM/OS family across an open-ended set of compatible
networks. Any number of workers can instantiate its capabilities. A worker
can run inside a node, in a separate process, or on another machine.
Switching CPU and GPU changes the execution backend while preserving the
warrior's identity.

This is the foundational soft3 architecture contract. MUST, SHOULD and MAY
describe requirements on conforming implementations. The model is adopted;
implementation coverage is recorded separately below. This document defines
ownership and compatibility, leaving concrete wire encodings and CLI commands
to their respective specifications.

## 1. vocabulary and cardinality

| concept | definition | relationship |
|---|---|---|
| VM / terrain | versioned instruction semantics, value model and machine ABI | usable by multiple OS environments |
| OS / union | versioned host operations, state-access ABI and protocol environment | instantiated by many networks |
| warrior | reusable target implementation integrating build, execution, proving, verification and supported deployment adapters for a VM/OS family | supports many compatible network instances and many workers |
| proof profile | exact statement semantics, proof format, verifier parameters, disclosure and supported operation limits | selected independently, subject to VM/OS compatibility |
| network instance | an identified instance of an OS/protocol, with genesis, domain and versioned rules | supplied as configuration to a compatible warrior |
| backend | concrete implementation of an operation using selected software and hardware | multiple backends can implement one warrior's capabilities |
| worker | a running instance of warrior capabilities with a backend, resource budget and job lifecycle | can serve jobs for multiple networks under explicit policy |
| node | participant owning network state, synchronization and admission/finality policy | can use local or remote workers |

Warrior names identify implementations such as Joy and Trisha. The network
name, process ID, CPU/GPU model and operator identity are separate properties.
An operator can launch many workers from one warrior implementation; this
does not require a central warrior process. A worker can execute many jobs
over its lifetime, with bounded concurrency.

The number of supported network instances is structurally unbounded: a new
compatible instance MUST be expressible as data. Each deployment still has
finite resource, connection and admission limits, which it MUST report and
enforce. Supporting a VM/OS family does not imply support for every VM/OS
or every future protocol revision.

## 2. five selections

Resolve these choices explicitly for each operation:

| selection | includes | independent variation |
|---|---|---|
| machine | VM semantics/version, field/value encoding, instruction and memory ABI | a different network can use the same VM |
| environment | OS host calls, state/read/write interface and protocol ABI/version | pure computation may use no OS environment |
| proof | statement kind, format/version, verifier parameters and disclosure | run-only work may request no proof |
| network | genesis/domain identity, rule versions, trust configuration and endpoint locations | pure computation may use no network |
| executor | warrior/version, backend/version, hardware, placement and budget | local CPU, local GPU or remote execution |

These are separate selections with compatibility constraints. They are not
an unrestricted Cartesian product. A warrior MUST describe the combinations
it supports and refuse incompatible ones before dispatch. For example, native
execution may support an instruction that its proof profile cannot prove.

Execution requires support from the warrior and selected backend. Network
admission additionally requires the network's policy to accept that operation
and proof profile. Neither capability advertisements nor a network label
constitute verification or admission evidence.

## 3. networks are open configuration

A warrior MUST accept an explicit descriptor for a compatible network without
adding its name to a source-code enum, creating a new warrior, or rebuilding
the binary. Bundled mainnet/testnet presets are conveniences; the same
descriptor validation applies to presets and caller-supplied instances.
Operators MAY restrict allowed networks through deployment policy.

The logical descriptor identifies the OS/protocol ABI, genesis commitment,
network domain/chain identity, applicable rule versions and proof policy.
Its concrete identity encoding belongs to the OS/protocol specification.
Trust anchors and endpoint locations are configured alongside that identity.
Credentials belong in private configuration, outside public identity data.

An endpoint is a location. Changing an endpoint MUST NOT silently change the
network. A display name such as mainnet/testnet is an alias. Genesis alone
may be shared by forks; the OS's domain/fork identity and trust rules must
disambiguate them. If a protocol cannot distinguish two forks, the adapter
must expose that limitation and apply its explicit fork policy.

A state snapshot is a root/height or equivalent within a network. Selecting
another snapshot does not create a new warrior or normally a new network.
Trident's existing `state`/`StateConfig` vocabulary names a network-instance
preset; specifications must distinguish that preset from a state snapshot.

Changing a compatible network's genesis, domain, endpoints or permitted
parameters is configuration. New opcodes, host calls, encodings or proof
semantics can require updated target/backend support. OS-specific parameter
schemas define the boundary; unsupported versions fail explicitly.

## 4. warrior, backend and worker

```text
Trident — language, types, shared compiler contracts
   │
   ├─ Joy — nox + supported OS adapters
   │    ├─ worker A: embedded CPU, network alpha
   │    ├─ worker B: isolated GPU, network alpha
   │    └─ worker C: remote backend, networks beta and gamma
   │
   └─ Trisha — Triton + supported OS adapters
        └─ independent workers and compatible network instances
```

This diagram describes permitted composition, not current GPU/remote support.
For each job a worker has an explicit network context or an explicit stateless
context. Worker concurrency and multi-network service are optional runtime
capabilities; a conforming implementation can begin with one active job.

Backend substitution MUST preserve the selected semantics and verification
contract. CPU/GPU traces, proof bytes and timings may differ where the profile
permits; accepted public results must satisfy the same statement. A fallback
must be disclosed and must respect requested budgets and placement policy.
Changing proof semantics requires explicit profile negotiation.

Worker placement has three forms: embedded library, isolated local process,
remote executor. The logical job/result contract is shared; transport and
isolation guarantees differ. A mode MUST refuse requested cancellation,
memory or time guarantees it cannot enforce. A backend crash or cancellation
cannot produce an accepted result through a weaker fallback.

One binary may host a node and embedded workers, or expose separate node and
worker modes. Standalone warrior CLIs serve development without node setup.
The architecture imposes no mandatory binary count and no mandatory daemon
for stateless build/run/prove/verify.

## 5. isolation and verification across networks

Every stateful job MUST bind the intended network identity, protocol/profile
versions, snapshot, program and public inputs according to its statement.
The scheduler retains its expected context independently of worker output.
Copying a root or network name into unverified metadata does not bind a proof.
The receiving node verifies against its own trusted context and policy.

Workers serving multiple networks MUST isolate state handles, witnesses,
credentials, cancellation and accounting. Identical labels, heights or job
IDs in different networks must not collide. Scheduling, caches and acceptance
records must include the relevant network/domain, program, profile and state
identity. Reward eligibility and replay protection remain network policy.

Immutable program artifacts may be reused across networks when their VM/OS
ABI and compilation inputs match. Network-specific constants or SDK/profile
changes participate in artifact identity. A public stateless computation
certificate may also be reusable when its exact statement permits; reuse
alone never establishes fresh work or authorization for another network.

Workers produce results. Verifiers establish the stated computation. Nodes
decide whether verified results authorize a state transition or reward.
This authority boundary holds even when all three run in one process.
Compilation correctness also requires its own validation: a machine-execution
proof alone does not prove that lowering preserved the source semantics.

## 6. ownership and specialization

| owner | contract |
|---|---|
| soft3 | this vocabulary, the five selections, open network cardinality and composition invariants |
| Trident | language/compiler semantics, warrior-facing API, target-package discovery and compilation interface |
| warrior implementation | supported target packages, SDK adapters, operation capabilities and developer CLI |
| VM / OS owner | authoritative machine/host ABI and protocol rules consumed by those adapters |
| proof-system owner | proof statement, format, verifier parameters and validation machinery |
| network/node product | concrete network policy, job scheduling, state admission, finality and rewards |
| backend implementation | operation execution and enforceable hardware/resource limits |

Warriors package version-matched adapters and SDK resources; machine and
network rule definitions retain one authoritative owner. Target declarations
must identify build, run, prove, verify and deploy capabilities independently.
Successful compilation, execution, proof generation and deployment are
different claims. A dry-run deployment plan establishes preparation only.

Nox reference lowering currently stays in Trident; Joy integrates nox and
Zheng. Trisha owns its Triton lowering and runtime integration. These choices
fit the same contract without requiring identical internal repository layouts.

The catalogue term warrior may cover mining tools as well. Trident compatibility
requires the actual supported compiler/runtime contracts. Mining/search is an
additional capability; a nonce-search tool need not implement arbitrary proving.

## 7. current implementation boundary

This specification does not claim that the following gaps are already closed:

| component | observed surface | remaining alignment |
|---|---|---|
| Joy | nox execution, restricted public execution certificates, target description | cyber is currently a stateless nox alias; build/deploy CLI are planned; open network binding and worker scheduling are future work |
| Cyber | local node binary and proposed worker contract | embedded Joy, remote workers, network verification and arbitrary compatible instance configuration remain integration work |
| Trident | warrior API, target discovery and runtime traits | some prose assumes an external binary and one battlefield; generic network descriptors and operation capabilities require alignment |

Concrete contracts:

- [Trident Warrior API](https://github.com/cyberia-to/trident/blob/master/reference/warrior-api.md).
- [Joy CLI](https://github.com/cyberia-to/joy/blob/main/specs/cli.md) and [Trident wiring requests](https://github.com/cyberia-to/joy/blob/main/.claude/trident-wiring.md).
- [Cyber worker specialization](https://github.com/cyberia-to/cyber/blob/master/specs/worker.md).
- [Cyber CLI](https://github.com/cyberia-to/cyber/blob/master/specs/cli.md) and [node composition](https://github.com/cyberia-to/cyber/blob/master/specs/node-product.md).
- [Trisha architecture](https://github.com/cyberia-to/trisha/blob/master/docs/explanation/architecture.md).

These are repository destinations; local branch changes require their normal
merge/publication before appearing at the linked default-branch locations.

## 8. conformance gates

1. Configure two compatible network instances, then a previously unknown
   third one, using the same installed warrior binary. No enum/source edits
   or rebuild may be required. Apply declared resource limits normally.
2. Preserve network identity across endpoint changes; reject an endpoint
   serving the wrong network. Reject unsupported VM/OS/proof combinations.
3. Run equivalent jobs on supported CPU/GPU backends; compare public
   semantics and verify each result under the same admitted proof profile.
4. Exercise embedded, isolated and remote modes only where advertised;
   refuse unenforceable limits instead of claiming equivalent isolation.
5. Serve different networks with overlapping labels/job IDs without leaking
   witnesses, using the wrong snapshot or duplicating acceptance/rewards.
6. Perform stateless development without node configuration; distinguish
   compiler success, execution, proof verification and network acceptance.

These are implementation gates, not a report of tests already passing.
Wire formats and concrete transport endpoints remain separately versioned.
Component specs may specialize this model but must preserve these invariants.
