# Neptune custom token v2

This is a complete bounded custom type predicate, not Neptune's native currency
or the PLUMB account-tree standard. The actual compiled program hash is its
type-script identity. Native public input is exactly the reversed kernel,
salted-input and salted-output Digests (15 fields); public output is empty.
The full SingleProof graph must authenticate the kernel and require this type
proof for the corresponding UTXOs. This policy has no fee or timestamp rule.

Every salted UTXO list is authenticated as the exact canonical Neptune 0.15.1
`BFieldCodec` encoding with Tip5 variable-length hashing. Parsing validates all
nested lengths, counts and complete consumption, including unrelated coins.
The predicate selects **every** coin whose type hash equals its own actual
program hash, never a prover-supplied selector. Each selected coin has exactly
seven state fields: `[2, amount_U32, authority_Digest_0..4]`. Token identity is
the pair `(program hash, full authority Digest)`. All selected coins in this
transaction must have the same authority; mixed-issuer transactions for this
program are intentionally rejected. At least one selected coin is required.

Input and output totals are checked U32 sums. Equal totals require no issuer
secret. Any change (mint or burn) requires the five-field secret whose
`Tip5.hash_10(secret[0..5],2,3,0,0,0)` equals the full committed authority.
The all-zero authority explicitly disables supply changes. Changing authority
is not an owner update: it changes token identity and mixed-identity transfers
are rejected. Initial issuance therefore needs a nonzero authorized issuer;
fixed-supply zero-authority assets need independently established genesis.

Resource bounds are part of this version's admission policy: each full encoded
list has at most4096 fields; at most64 UTXOs per list and64 coins per UTXO.
All lengths and counts are U32 and checked before traversal. Arbitrary state of
unrelated types is preserved/authenticated within the total list bound. RAM
`[8000000,8004110)` and `[8010000,8014110)` is internal scratch. The witness is
`input_length, input_encoding, output_length, output_encoding`, followed by the
issuer secret only for a supply change. No claimed amount or config is accepted
outside the authenticated encoding.

Source and independent hand assembly have different program hashes, hence
different encoded type hashes and resulting public salted-list digests. Their
benchmark vectors must disclose this implementation-bound statement identity;
amounts, authority, locks, salts, unrelated coins and expected outcome remain
identical and derive from the same independent canonical-codec oracle.
