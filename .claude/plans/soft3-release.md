# trident 0.2.0 — the soft3 release

**STATUS: shipped 2026-09-08.** trident-lang 0.2.0 + cyber-joy 0.2.1 on
crates.io, GitHub releases cut, the whole harness republished
(strata-nebu 0.1.1, cyber-hemera 0.3.1, cyber-lens 0.1.3, cyber-nox
0.2.0, zheng 0.2.0, bbg 0.2.0). M0–M7 done; residuals recorded in
CHANGELOG. Kelvin: 512K → 500K (see roadmap).

**One sentence:** trident compiles to nox by default with soft3 as its
infrastructure — strata algebra and hemera hashes in the compiler, and a new
warrior **joy** (the cyber battlefield, built the way trisha is built) doing
execute · prove · verify · deploy — so `trident prove` produces a zheng
proof. 0.2.0 ships only when the whole scope, zheng openings and bbg
included, is done. The proof is the meat; no gates, no --experimental.

Supersedes and absorbs: `nox-target.md` (Phase 1 executed), `nox-api-alignment.md`
(done on branch `feat/nox-api-alignment`). Both get deleted on sign-off.

## Current state (audited 2026-09-07)

| fact | state |
|---|---|
| `vm/nox/target.toml` | exists — hash=Hemera(8,8), ext degree 3, cost=reductions, `status: level=3, cost_model=false, tests=false` |
| `src/ir/tree/lower/nox.rs` | 706 LOC NoxCompiler, Phase-1 surface (literals, let, arith, cmp, if, hash; **no** loops/calls/structs/assign) |
| deps | `cyber-nox` + `strata-nebu` already in Cargo.toml (path); build green on `feat/nox-api-alignment` |
| vocabulary | commit 7acc4f8 already says "zheng proofs on the nox path" |
| nox (cyber-nox 0.1.2) | reduce + Tracer + jets + brakedown_look; sits on strata-nebu; `ask()` = 7 fields of a cyberlink |
| zheng 0.1.2 | sumcheck core green; **blocked: axis/hash/look opening derivation** (registry state = blocked) |
| bbg 0.1.2 | live; **QueryProof has no serde** (lens Commitment/Opening are internal) |
| hemera 0.3.0 | Poseidon2 sponge, particles, trees, verified streaming |
| trident field/ | own goldilocks/poseidon2 — duplicates nebu + hemera; **to be replaced, not kept** |
| babybear/mersenne31 | NOT ours and NOT a problem: fields of foreign targets (risczero 2^31−2^27+1, sp1 2^31−1) in the multi-VM registry; used nowhere outside field law-tests + target configs. They stay inert target-side properties — never enter strata |

The integration is not a graft — the design anticipated it. What's missing is
completion, proving, identity — and the removal of every parallel
implementation: trident accumulates soft3 as default.

## Milestones

### M0 · land the alignment (done, needs merge)
PR `feat/nox-api-alignment` → master. Uncommitted Cargo.toml/lock tidied first.

### M1 · full language surface → nox (3–4 sessions)
NoxCompiler Phase 2: bounded `for` (unroll when bound is static; recursive core
otherwise), multi-function calls (battery in subject), structs/arrays/tuples
(cons-trees), mutable assignment (subject edit), sponge/Merkle builtins → nox
hash pattern + jets. Builtin-sync rule holds across the 4 places
(reference/language.md · typecheck · tir/tree · cost). Unit tests per mapping;
`tests/nox_surface.rs` compiles every language feature.

### M2 · honest cost model (1–2 sessions)
`[cost] tables=["reductions"]` becomes real: static reduction/jet counts per
builtin, `trident build --target nox --cost` prints the proving bill.
`trident bench` grows a nox column (unit: reductions, not cycles — no fake
equivalence). target.toml flips `cost_model=true, tests=true`, level 4.

### M3 · joy is born — run + verify, end to end (2–3 sessions)
New warrior **joy** (`~/cyber/joy`, sibling of trident, built exactly the way
trisha is: thin Rust binary, `trident` via `path = "../trident"`, implements
the `Runner`/`Prover`/`Verifier`/`Deployer` traits over ProgramBundle).
Battlefield: cyber — terrain nox. Deps live in joy, not in trident-lang:
cyber-nox, zheng, bbg, cyber-hemera, strata-nebu. trident's CLI already
delegates run/prove/verify to the resolved warrior — joy slots into the
battlefield registry beside trisha.
M3 scope: `joy run` (nox reduce with Tracer) and `joy verify` by re-execution.
Differential harness: every `benches/references/` program runs on both Triton
(trisha) and nox (joy) — outputs must agree. This is the release's
correctness anchor.

### M4 · zheng prover — the meat (4–8 sessions, no gate)
`joy prove`: nox trace → zheng witness → zheng proof; `joy verify` checks the
proof without re-execution. **The blocker is the work:** axis/hash/look
opening derivation in zheng gets solved inside this release — it is the
release. Order of attack: wire the pipeline for Layer-1-only programs first
(forces the axis/hash openings), then look (meets M6's bbg serde). 0.2.0 does
not ship until `joy verify` accepts a zheng proof for every differential-
harness program. No --experimental, no honest-dashes fallback: the dash today
IS the release note; 0.2.0 replaces it with the number.

### M5 · soft3 replaces field/ — one algebra, one hash (3 sessions)
No parallel implementations survive. trident's `field/` is **replaced** by the
stack, not cross-tested against it:
- `goldilocks.rs` → deleted; `Field` is `strata-nebu` (already a dep).
- `poseidon2.rs` → deleted; hashing is `cyber-hemera` everywhere — compiler
  internals, content addressing, the `hash` builtin, proof params. One
  Poseidon2 on the planet, and it is hemera's.
- `babybear.rs` / `mersenne31.rs` → stay: they are foreign-target properties
  (risczero, sp1), not stack algebra. Our thing is Goldilocks; strata never
  learns these fields. The `PrimeField` trait shrinks to what foreign targets
  need; the Goldilocks path speaks strata's trait directly.
- migration is bit-exact-or-explicit: any digest that changes value is a
  breaking change called out in the CHANGELOG, not silently absorbed.

Then identity lands on the real hash: content-addressed code graduates from
"hash of normalized AST" to **the hemera particle of the normalized AST** —
`trident deploy` emits particle + cyberlinks (name→hash is a link,
certificate→hash is a link). Memoization comes free: nox `ask()` computes
`order_axon = H(formula, object)` — deployed trident functions are globally
memoizable by construction.

### M6 · bbg state — the look pattern opens (2 sessions + small bbg PR)
nox pattern 17 (look) reads bbg with proof, in joy. Needs upstream: serde for
`QueryProof`/`Commitment`/`Opening` in bbg/lens (small, unblocks the soft3 SDK
too). trident side: `os.state.read` lowers to look; bundle declares state
deps. **In-scope for 0.2.0, no slipping** — the scope is discrete and
completes in this release.

### M7 · truth in public (1 session)
- README: nox is the default target — "produces a zheng proof; STARK on
  Triton via `--target triton`"; quick start shows the default path; the
  "minimal dependencies" principle rewritten: soft3-native by default,
  nothing outside the stack.
- trident.pink hero line gains the same dual-proof truth (only after M3 green).
- soft3 `status.md`: trident row state, ".tri → .nox" becomes verified fact;
  zheng row updates per M4 outcome.
- CHANGELOG + GitHub release 0.2.0; conformance note in soft3.

## Architecture decision (settled by owner)

**The trisha pattern, faithfully: a new warrior — joy — for the cyber
battlefield.** trident stays the weapon (compile, cost, audit; its algebra and
hash become strata + hemera by default, nox the default target so
`trident build main.tri` emits `.nox`). joy is the warrior: a thin external
binary owning execute · prove · verify · deploy on nox, carrying the heavy
deps (cyber-nox for execution, zheng, bbg, cyber-hemera). Triton keeps
trisha; cyber gets joy; the ProgramBundle boundary stays clean. `cargo
install trident-lang` + `cargo install joy` is the full kit; the README's
"minimal dependencies" principle is rewritten in M7: minimal means *no
dependency outside the soft3 stack*.

## Out of scope (0.3.0+)

os/cyber union target (os.signal/os.neuron via mudra+cybergraph), radio/cell
integration, foculus, jali/FHE path, nox native compile (wasm/arm64 — separate
plan in nox repo).

## Verification (per repo rules)

Every milestone: `cargo check` zero warnings · `cargo test` · `trident bench`
no regressions · `trident audit` holds · differential triton×nox agreement
(from M3). Branches `feat/*`, PRs, no direct master commits.

## Estimate

Full scope M0–M7, no gates: **~16–22 sessions.** The spread is M4 — zheng
openings are research-shaped; everything else is engineering-shaped. 0.2.0
ships when the last milestone is green, and not before.

## Settled

- Warrior **joy** for the cyber battlefield, trisha-style — not features in
  trident. (owner, 2026-09-07)
- 0.2.0 ships only with M4 complete — the proof is the meat, no exceptions.
  openings + bbg are inside 0.2.0; the scope is discrete. (owner, 2026-09-07)
- Version: 0.2.0. (owner, 2026-09-07)
- babybear/mersenne31: foreign-target fields (risczero, sp1), inert, never
  enter strata — investigated, nothing wrong. (2026-09-07)
