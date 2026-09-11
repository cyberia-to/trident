# Runtime and OS Reference

[Targets](targets.md) · [VM contracts](vm.md) · [Standard library](stdlib.md)

A runtime contract describes the environment around a VM: transaction
layout, state access, accounts and external operations. It is distinct from
the VM's instruction set and from a deployment instance's endpoint.

## What is implemented

| Surface | Implementation and limitations |
|---|---|
| `os.state.read` | Compiler builtin lowered to nox state lookups; not a portable OS source library and not authenticated public proof support |
| `os.neptune.*` | Trisha-owned Neptune SDK source and transaction policy |
| `os.neuron`, `os.signal`, `os.event` | Design vocabulary; no portable source implementations in Trident |
| Cyber network SDK and state loading in Joy | Not implemented; `cyber` currently aliases stateless nox |
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

The existence of `os.state.read` proves only that the compiler can express a
state lookup. A runner needs a supplied state root and provider, and a proof
needs an authenticated state relation. These are separate requirements.
Joy rejects CLI state requests and stateless execution of bundles marked
`reads_state`. Its public Zheng execution certificates do not support state,
secrets or calls. Legacy state-proof acceptance failures remain release
requirements; they are not covered by passing public execution tests.

## Design vocabulary

Neurons, signals and tokens describe a proposed portable model for identity,
transactions and assets. This vocabulary does not establish implementations
of `os.neuron.id`, `os.neuron.auth`, `os.signal.send`, event emission or generic
storage across every catalog OS. A future portable contract needs explicit
semantics, capability failures, implemented target mappings and conformance
tests before it becomes a language guarantee.

See the [ownership review](../docs/explanation/target-ownership.md) for the
migration rationale and [Warrior API](warrior-api.md) for resource discovery.
