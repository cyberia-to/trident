# Guest intrinsic integration review

Scope: the `.tri` compiler changes based on combined profile source
`79cb4aef62f77eea7052fa2166af4b93ae086aa2`. This records source review;
executable acceptance and final source identity belong to the accompanying
validation receipt. It does not establish compiler self-hosting or a proof.

- Determinism: the declaration table uses absolute function IDs. Monotonic
  insertion into a persistent tree retains a zero value for absent IDs; final
  name/import resolution selects the ID before intrinsic lookup. Neither
  function basenames nor transitive requirements infer an intrinsic identity.
  Namespace policy compares exact bytes with a bounded rolling U32 window.
  Shared long-spelling comparison checks each byte without a modular hash.
- Types: all known declarations validate exact arity, ordered parameter types
  and complete return descriptors, including the canonical two-U32 split tuple.
  Private and replaced declarations are checked before publication. Known ABI
  identity and executable lowering are separate decisions: an unused, typed
  declaration may remain unavailable to the native backend.
- Errors: final attribute identity/span wins. Invalid namespace or ABI reports
  invalid binding; unknown identity and unsupported forms retain the full
  attribute span. Reachable unsupported operations preserve the original call
  span and original package module index through dependency planning. Existing
  lexical/parse precedence remains in the header and attribute readers.
- Reachability: bodyless records expose only an empty call-graph flow spine;
  their zero expression/control slots cannot decode as an executable body.
  Intrinsic calls become builtin AST operations and create no ordinary function
  edge. Entry validation separately rejects intrinsic entries. Only reachable
  ordinary bodies undergo unsupported-operation preflight.
- Visibility and purity: final public callable exports and immutable direct-use
  bindings govern full and short aliases. Ordinary replacements keep their
  bodies, including ordinary functions named assert. Purity checks the selected
  source member before lowering its identity. Wrappers retain ordinary call
  semantics and argument ordering.
- Resource bounds: table insertion checks ID range, registered kind, logical
  capacity and monotonicity. Source scanning, tree traversal, expression preflight
  and body traversal remain bounded. The table is compiler-owned with private
  fields; no public noun decoding path admits an unchecked intrinsic table.
  Exact spelling helpers receive lexer-validated spans and at most nine bytes.
- Integration: header state keeps frequently updated token/error fields at the
  existing indices. Cold declaration metadata resets when a declaration ends.
  Package/header/parser profile propagation from the preceding delivery remains
  intact; generated compiler profiles still require the final Noun → Noun entry.
- Collections: indexed reads consume two tree levels per continuation, with
  direct terminal reads at heights one through three. Capacity remains a Field,
  including the height-32 case; only capacities at most 2^31 narrow to U32.
  The offset remains relative to the selected subtree, and pair leaves stay
  opaque. Byte reads select constant masks and inverse powers for the four
  packed lanes. The typed Bytes handle still owns bounds and word validation.
  Encoding, padding, validation visit charges and public quotas are unchanged.
- Evidence boundaries: whole-source capacity, legacy import remaps, complete
  C2/C3 reproduction and native Zheng proofs have separate gates. Historical
  allocation failures and failed test runs must remain alongside final receipts.
