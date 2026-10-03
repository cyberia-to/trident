# Native private execution

Implementation contract for the owner-authorized native proving delivery.

Joy executes nox programs and uses Zheng to prove their verifier-derived CCS.
The private profile uses Zheng's native finite-ring MPC-in-the-head protocol over
Goldilocks and Hemera. Its relation comes from the public canonical program and
subject shape. Its witness consists of private call inputs and intermediate
values. The verifier receives a public statement and a randomized proof.

`joy-nox-zheng-private-execution-v1`, header `JOYZH001`, binds the canonical
program, public inputs/outputs, selected reduction count, budget, source identity,
program name, state ABI and authenticated state identity. The private backend
fixes its parameters in Zheng's `specs/native-private-ccs.md`. Artifact size is
bounded at256MiB. Canonical decoding, exact lengths and proof-shape validation
precede expensive verification. Retired JOYZK envelopes are separate formats.

`joy prove --secret ...` selects private proving. `--zk` selects it even when the
program has no secret call. `Prover::prove` selects the private profile for secret
inputs. Explicit public proof APIs reject secrets because their witness is public.
Private verification needs no secret input and performs no native re-execution.
Expected source/build, public input/output, budget and state root are checked when
supplied. Supplying secret input during proof verification is an error.

Every scalar input is a canonical Goldilocks value. Successful native execution
and proving consume exactly the supplied active-call witness stream. Inactive
calls consume no value. Missing/excess inputs fail without printing secret data.
The call continuation and all uses of its result share the same constrained wire.

State proving authenticates all ten public BBG tables before deriving relation
constants. The relation constrains each selected namespace, index, value and root.
The certificate remains public; the query coordinates and intermediate witness
are hidden. Program source, public result and reduction count remain public.

Proof publication happens only after successful native/relation agreement and
complete serialization. Existing output files survive failure. CLI replacement
requires `--force`; otherwise publication refuses an occupied output path.

Acceptance requires native secret/stateless/state proofs, separate-process
verification, invalid witness and tampered claim rejection, fresh proof randomness,
privacy analysis of opened views, and an absent foreign runtime dependency graph.
Evidence and measured costs belong in `audit/`, never in this contract.
