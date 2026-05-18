# PLUMB — Token Framework

Pay, Lock, Update, Mint, Burn. The shared framework for all Trident
token standards.

See the [Gold Standard](../docs/explanation/gold-standard.md) for design
rationale, proof composition examples, and skill architecture.

---

## 1. The 10-Field Box

Every PLUMB box contains exactly 10 field elements. The first 5 are
shared across all standards. The last 5 are standard-specific.

```
box = (id, value, nonce, auth_hash, lock_until,
       standard_field_5, ..., standard_field_9)
```

| Position | Framework name | TSP-1 (Coin) | TSP-2 (Card) |
|----------|---------------|--------------|--------------|
| 0 | id | `account_id` | `asset_id` |
| 1 | value | `balance` | `owner_id` |
| 2 | nonce | `nonce` | `nonce` |
| 3 | auth_hash | `auth_hash` | `auth_hash` |
| 4 | lock_until | `lock_until` | `lock_until` |
| 5 | *(standard)* | `controller` | `collection_id` |
| 6 | *(standard)* | `locked_by` | `metadata_hash` |
| 7 | *(standard)* | `lock_data` | `royalty_bps` |
| 8 | *(standard)* | `0` (reserved) | `creator_id` |
| 9 | *(standard)* | `0` (reserved) | `flags` |

### Shared Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | Field | Unique identifier (pubkey hash for accounts, asset hash for items) |
| `value` | Field | Primary value (balance for coins, owner for cards) |
| `nonce` | Field | Monotonic counter — increments on every state change |
| `auth_hash` | Field | Hash of authorization secret (see [Authorization Model](#3-authorization-model)) |
| `lock_until` | Field | Timestamp lock — operations blocked until this time (0 = unlocked) |

See [TSP-1 Coin](tsp1-coin.md) and [TSP-2 Card](tsp2-card.md) for
standard-specific fields.

---

## 2. Token Config — 10 field elements

Shared by all PLUMB standards. Every operation verifies the full config
hash and extracts its dedicated authority and hook.

```
config = hash(admin_auth, pay_auth, lock_auth, mint_auth, burn_auth,
              pay_hook, lock_hook, update_hook, mint_hook, burn_hook)
```

### Authorities (fields 0-4)

| Field | Description |
|-------|-------------|
| `admin_auth` | Admin secret hash. `0` = renounced (permanently immutable) |
| `pay_auth` | Config-level pay authority. `0` = account auth only |
| `lock_auth` | Config-level lock authority. `0` = account auth only |
| `mint_auth` | Config-level mint authority. `0` = minting disabled |
| `burn_auth` | Config-level burn authority. `0` = account auth only |

### Hooks (fields 5-9)

| Field | Description |
|-------|-------------|
| `pay_hook` | External program ID for pay logic (`0` = none) |
| `lock_hook` | External program ID for lock logic (`0` = none) |
| `update_hook` | External program ID for update logic (`0` = none) |
| `mint_hook` | External program ID for mint logic (`0` = none) |
| `burn_hook` | External program ID for burn logic (`0` = none) |

### Authority Semantics

| Operation type | Auth = 0 | Auth != 0 |
|----------------|----------|-----------|
| Account ops (pay, lock, burn) | Account auth only (permissionless) | Dual auth: account + config authority |
| Config ops (mint) | Operation disabled | Config authority required |
| Config ops (update) | Renounced (permanently frozen) | Admin authority required |

---

## 3. Authorization Model

### Auth Hash

Every box stores `auth_hash = hash(secret)`. To authorize an operation,
the prover divines the secret and the circuit verifies
`hash(secret) == box.auth_hash`. The secret is never revealed — only
proven to exist. Any preimage scheme works (private key, multisig hash,
biometric hash), providing account abstraction at the protocol level.

### Dual Authorization

When a config authority (e.g. `pay_auth`) is non-zero, the operation
requires two proofs: the account holder's auth hash verification AND a
proof that the config authority was satisfied. Both must be valid for the
operation to succeed.

### Controller Authorization

When `box.controller != 0` (TSP-1), every operation additionally
requires a composed proof from the controller program. Enables
program-controlled boxes: fund collateral, escrow, protocol
treasuries.

---

## 4. Nonce and Replay Prevention

Every box contains a monotonic `nonce` counter. On every state change,
`nonce += 1`. The circuit rejects any proof where the new nonce is not
exactly `old_nonce + 1`.

On state-changing operations, a nullifier is emitted:

```
nullifier = hash(id, old_nonce)
```

The nullifier uniquely identifies the consumed box state. The consensus
layer rejects duplicate nullifiers, preventing replay of old proofs
against current state.

---

## 5. Proof Envelope

Every PLUMB operation follows this verification pattern:

1. Divine 10 config fields from witness
2. Hash config fields, assert result matches public `config_hash`
3. Extract this operation's authority and hook from config
4. Verify authorization (account auth, dual auth if authority != 0)
5. Verify time-lock (`current_time >= lock_until`) where applicable
6. Apply state transition (standard-specific constraints)
7. Update Merkle root (old box → new box)
8. Emit nullifier for consumed box
9. Emit public I/O (op code, old root, new root, amounts, config digest)

The proof envelope guarantees that every operation is bound to an
authentic config and that authorization is verified before any state
change.

---

## 6. The Five Operations

| Op | Name | Purpose | Modifies state | Modifies supply/count |
|----|------|---------|:--------------:|:---------------------:|
| 0 | Pay | Transfer value or ownership | Yes | No |
| 1 | Lock | Time-lock box until future timestamp | Yes | No |
| 2 | Update | Change token configuration | No | No |
| 3 | Mint | Create new value or asset | Yes | Yes (+) |
| 4 | Burn | Destroy value or asset | Yes | Yes (-) |

### Shared Constraints

These constraints apply to all PLUMB standards. Standard-specific
constraints (balance arithmetic, uniqueness proofs, flag checks) are
specified in [TSP-1](tsp1-coin.md) and [TSP-2](tsp2-card.md).

**Op 0 — Pay:**
Config verified. Account auth required. Dual auth if `pay_auth != 0`.
`current_time >= lock_until`. Nonce incremented. Supply/count unchanged.

**Op 1 — Lock:**
Config verified. Account auth required. Dual auth if `lock_auth != 0`.
`new_lock_until >= old_lock_until` (extend only, never shorten).
Nonce incremented. Supply/count unchanged.

**Op 2 — Update:**
Config verified against old config hash. Admin auth required.
`admin_auth != 0` (renounced configs cannot be updated). State root
unchanged (`old_root == new_root`). New config fields hashed and emitted
as new `config_hash`.

**Op 3 — Mint:**
Config verified. `mint_auth != 0` (zero = minting disabled). Mint
authorization verified against `config.mint_auth`. New box inserted
into tree. Supply/count incremented.

**Op 4 — Burn:**
Config verified. Account auth required. Dual auth if `burn_auth != 0`.
`current_time >= lock_until`. Nonce incremented. Box removed or zeroed.
Supply/count decremented.

---

## 7. Merkle Tree

- Binary tree of depth `TREE_DEPTH` (e.g. 20)
- Leaf node: `hash(box.field_0, ..., box.field_9)` — box commitment hash
- Internal node: `hash(left[0..5], right[0..5])`
- Root is the public state commitment

Every operation proves a Merkle inclusion path from the box to the
root, applies the state transition, and produces a new root. The
verifier checks that the old root matches the current on-chain state
and that the new root is correctly derived.

---

## 8. Global Public State

Each token maintains these public values on-chain:

| Field | Type | Description |
|-------|------|-------------|
| `state_root` | Digest | Merkle root of all boxes |
| `supply` or `count` | Field | Total value (TSP-1: sum of balances) or total items (TSP-2: number of assets) |
| `config_hash` | Digest | Token configuration commitment |
| `metadata_hash` | Digest | Token or collection metadata commitment |
| `current_time` | Field | Block timestamp (used for time-lock checks) |

The `supply`/`count` field is standard-specific:
- TSP-1: `supply` = sum of all balances, enforced per-operation
- TSP-2: `asset_count` = number of assets in tree

---

## 9. Hook System

Each operation has a dedicated hook slot in the config. Hooks are
external ZK programs that compose with the token proof.

| Hook | Triggered by | Example use cases |
|------|-------------|-------------------|
| `pay_hook` | Every pay | Whitelist/blacklist, transfer limits, compliance, royalties |
| `lock_hook` | Every lock | Maximum lock duration, lock rewards |
| `update_hook` | Every config update | Multi-sig requirement, timelock on upgrades |
| `mint_hook` | Every mint | Cap enforcement, vesting schedule, KYC gate |
| `burn_hook` | Every burn | Minimum burn amount, burn tax, audit trail |

When `hook == 0`, no external proof is required.

When `hook != 0`, the token circuit proves the state transition is valid
and that the config (including hook reference) is authentic. The verifier
then composes the token proof with the hook program's proof. Both must
be valid. Public I/O is shared across sub-proofs, ensuring consistency
(same amounts, same accounts, same timestamps).

See [Skill Reference](skills.md) for the 23 standard hook programs.

---

## 10. Security Properties

These properties hold for all PLUMB standards. Standard-specific
properties (balance non-negativity, uniqueness, flag enforcement) are
listed in [TSP-1](tsp1-coin.md) and [TSP-2](tsp2-card.md).

1. **Replay prevention** — monotonic nonce + nullifiers `hash(id, old_nonce)`
2. **Time-lock enforcement** — `current_time` from block timestamp, checked before pay/burn
3. **Lock monotonicity** — locks can only extend, never shorten
4. **Config binding** — every operation verifies the full config hash
5. **Account abstraction** — `auth_hash` accepts any preimage scheme
6. **Irreversible renounce** — `admin_auth = 0` permanently freezes configuration
7. **Config-state separation** — Update (Op 2) cannot modify the Merkle tree
8. **Hook composability** — hooks are bound to config hash, composed at verification time
9. **Symmetric authority** — every operation has a dedicated authority field and hook field
10. **Safe defaults** — `mint_auth = 0` = minting disabled; other auths `= 0` = permissionless
11. **No approvals** — no allowances, no `transferFrom`, no approval phishing attack surface

---

## See Also

- [TSP-1 — Coin Standard](tsp1-coin.md) — divisible assets, `sum(balances) = supply`
- [TSP-2 — Card Standard](tsp2-card.md) — unique assets, `owner_count(id) = 1`
- [Gold Standard](../docs/explanation/gold-standard.md) — design rationale and philosophy
- [Skill Reference](skills.md) — 23 composable token skills (hook programs)
- [Standard Library: Token Infrastructure](stdlib.md#layer-05-token-infrastructure) — `std.token`, `std.coin`, `std.card`, `std.skill`
