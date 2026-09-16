# PLUMB v2: complete-digest authorization and atomic transitions

Version2 is the default executable Coin/Card example contract. Version1 is
retained only under `examples/experimental/neptune/plumb-v1`; its Field-sized
authorization and incomplete transitions are not production policy. The v2
program names, program hashes, commitments and witnesses are intentionally
incompatible. Five operations remain Pay0, Lock1, Update2, Mint3, Burn4; all
other opcodes reject.

## Authorization and config

Let `H` be native Tip5 hash of exactly10 fields and `P(a,b)=H(a[0..5],b[0..5])`.
All authorities are complete native five-field Digests. A five-field secret s
proves `authority = H(s[0],...,s[4],2,1,0,0,0)`. The all-zero Digest is reserved
and explicitly fails every required-authority check; it is never accepted on
an assumption about whether zero has a preimage. Optional dual authority is
skipped only for this exact zero Digest.

Config contains `admin,pay,lock,mint,burn: Digest` followed by five Field hook
registry identifiers. Define:

```
a = P(admin,pay)
b = P(lock,mint)
c = P(burn,H(pay_hook,lock_hook,update_hook,mint_hook,burn_hook,2,2,0,0,0))
config = P(P(a,b),c)
```

The public config commitment is a complete Digest. Config updates authenticate
all old fields, require nonzero old admin authority, authenticate all new
fields against a new public Digest, and preserve the state root.

## Leaves and identifiers

Coin leaf:

```
core = H(id,balance,nonce,lock_until,controller,locked_by,lock_data,2,1,0)
leaf = P(core,auth_digest)
```

Card leaf:

```
core = H(id,owner_id,nonce,lock_until,collection_id,metadata_id,royalty_bps,flags,2,2)
leaf = P(core,P(auth_digest,creator_digest))
```

Card's creator is the complete mint-authority Digest, checked at mint and
immutable afterwards. The historic `metadata_hash` field name now denotes an
opaque Field registry identifier; it is not a full cryptographic commitment.
Other Field hook/controller/owner/collection IDs likewise identify entries in
an externally authenticated registry. Authority checks never authenticate a
program or owner using a truncated digest.

IDs are nonzero direct indices below2^20. Empty slots contain
`H(0,0,0,0,0,0,0,0,2,0)`. Every update uses the same20 sibling Digests for both
old and new leaves. The siblings are read as ordinary secret fields, in leaf
to root order. Pay updates the sender first and the distinct receiver second
against that intermediate root. Nonmembership at Card mint and empty-leaf
replacement at Card burn enforce uniqueness and deletion. Numerical balances,
count/supply, timestamps and nonces fit U32 before and after arithmetic.

## Operation requirements

- Pay: config, account authority and optional dual authority; lock elapsed;
  nonnegative sender remainder and bounded recipient sum; both updated roots
  bound atomically; Card transfer flag and immutable fields preserved.
- Lock: config/account/optional dual authority; lock never decreases; nonce
  increments; Card lock flag; only the permitted leaf fields change.
- Update: Coin config update or Card config/metadata update; admin nonzero for
  config changes, owner authority and update flag for Card metadata changes;
  unchanged fields/root enforced according to branch.
- Mint: nonzero configured mint authority; exact bounded supply increment;
  Coin recipient increment or empty Card slot; Card creator equals mint
  authority, royalty<=10000 and mintable flags; cap bound to authenticated
  public collection metadata (ten fields with version2 at slot7).
- Burn: account/optional dual authority, elapsed lock and Card burn flag;
  bounded balance/supply reduction; atomic Coin balance update or Card deletion.

Public event output is tag then declaration-order fields, including all Digest
coordinates. Nullifier commitments bind consumed identifier and nonce. The
outer state verifier rejects replays and binds supplied old roots, supply,
config, metadata and timestamp to trusted current state.

## Composition boundary

These are custom account-tree state-transition predicates, not Neptune native
UTXO type scripts. A hook/controller registry ID requests external proof
composition; printing it alone does not authenticate that external program.
The integrating verifier must resolve the authenticated registry, verify every
requested proof against the same action, and bind trusted state/time/genesis.
The standalone transition proof does not claim these obligations are discharged.
