# Runtime and OS Reference

[Targets](targets.md) · [VM contracts](vm.md) · [Standard library](stdlib.md)

A runtime contract describes the environment around a VM: transaction
layout, state access, accounts and external operations. It is distinct from
the VM's instruction set and from a deployment instance's endpoint.

## What is implemented

| Surface | Implementation and limitations |
|---|---|
| `os.state.read` | Compiler builtin lowered to nox lookups; Joy authenticates supplied public BBG certificates and binds the lookup to execution |
| `os.neptune.*` | Trisha-owned Neptune SDK source and transaction policy |
| `os.neuron`, `os.signal`, `os.event` | Design vocabulary; no portable source implementations in Trident |
| Joy state loading | Authenticated public BBG certificate files, including bounded hidden query coordinates over public tables |
| Live Cyber network SDK | Not implemented; `cyber` aliases the supported nox runtime without network synchronization or deployment |
| Other catalog OS entries | Discovery/design records, not available runtime modules |

There are no implemented portable `.tri` OS modules hidden in Trident's
catalog. The old `os/` directory contained network descriptions and state
presets. Those are not a cross-platform runtime library.

## Namespaces and physical resources

| Namespace | Meaning | Physical owner |
|---|---|---|
| `std.*` | Portable algorithms, subject to explicit intrinsic requirements | Trident `lib/std/` |
| `vm.*` | Language intrinsic contracts | Trident `lib/vm/` |
| `vm.triton.*` | Explicit Triton ABI bindings | Trisha `lib/vm/triton/` |
| `os.<network>.*` | Runtime-specific SDK | Owning warrior's `lib/os/<network>/` |

Import names remain stable across physical layout changes. The owner's target
package supplies versioned module contents. A package may provide an SDK
without claiming that every program in it is executable, provable or ready
for submission; those claims need separate checks.

Trisha owns Neptune kernel and UTXO structures, lock policy, recursive
verification helpers and transaction programs. `lock_and_read_kernel` is
Neptune policy and belongs in `os.neptune.auth`, not a generic authorization
helper. Triton Merkle witness streams likewise do not belong to a portable
runtime abstraction.

## Protocol and deployment data

Trisha's `networks/neptune/target.toml` defines the runtime contract and
`networks/neptune/states/` contains deployment presets. Trident retains only
an owner reference under `catalog/os/neptune/`. Compiler and CLI consumers
use the packaged owner data, including installed operation outside a sibling
checkout.

`UnionConfig` names the VM and binding prefix. `StateConfig` names a deployment
instance and associated network data. Deployment state is excluded from the
package's compilation identity; compile-time network bindings and module
contents are included. A user endpoint override is configuration, not a
reason to duplicate the protocol SDK.

Packaging or registry publication must not be reported as successful chain
submission. Confirm the installed owner's deploy capability and the command's
actual result.

## State and proof status

`os.state.read` lowers through active imported and local helper calls. The
compiler propagates a hidden state root to stateful callees and marks the
selected entry bundle with `reads_state`; stateless call subjects are unchanged.
Joy supplies the root and provider from an authenticated public BBG certificate
selected with `--state`. Stateful bundles require that certificate; stateless
execution does not silently supply an unchecked root.

`JOYST001` proves public state execution, binding active namespace/key/value
coordinates and all four root limbs to the same verifier-derived Zheng relation.
`JOYZK003` proves private inputs and query coordinates using Trisha's native
Triton checker. Hidden queries require all ten authenticated public dimension
tables and retain the2048-field/32768-gate limits. The database remains public.
`JOYEXEC2` is the separate stateless public execution certificate; it discloses
its witness and does not accept private calls. Proof verification checks the
expected program, public input/output, selected cost and relevant state root.

Dynamic continuations/variable noun shapes, a private database and live state
synchronization remain outside the production proof contract. The experimental
tagged relation is not used by these formats. See
[Zheng execution contract](../../zheng/specs/execution.md) and
[Joy CLI](../../joy/specs/cli.md) for exact admission and disclosure requirements.

## Design vocabulary

Neurons, signals and tokens describe a proposed portable model for identity,
transactions and assets. This vocabulary does not establish implementations
of `os.neuron.id`, `os.neuron.auth`, `os.signal.send`, event emission or generic
storage across every catalog OS. A future portable contract needs explicit
semantics, capability failures, implemented target mappings and conformance
tests before it becomes a language guarantee.

See the [ownership review](../audit/target-ownership.md) for the
migration rationale and [Warrior API](warrior-api.md) for resource discovery.
