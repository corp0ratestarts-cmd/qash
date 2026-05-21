# QASH Sharded Protocol
## `docs/spec/12_sharded_protocol.md` — Protocol Version 1.2 Draft

> **Status:** Derived engineering specification. This file incorporates the
> sharding design requirements recovered from PR #93. It is not a genesis-lock
> change until the corresponding constants, replay vectors, and formal proof
> coverage are reviewed.

---

## §12.1 — Scope

Sharding is part of the QASH protocol structure, not merely an implementation
module. A shard is a deterministic execution partition whose public outputs are
committed into the epoch's global public transcript through an Epoch Finality
Beacon (EFB).

The sharded protocol preserves the existing QASH invariants:

- one-shot genesis-pinned rules
- no governance-controlled shard count changes
- deterministic cross-ISA execution
- no proposer trust for finality
- no raw transaction or graph publication in Domain A
- absorbing halt on invariant violation

Domain A observes only commitments. Domain B performs admission, transport,
private execution, and proof generation.

---

## §12.2 — Shard Assignment

Validators are assigned to execution shards by a deterministic Domain A function:

```
assign_shard(epoch_seed, validator_id, shard_count, bond_weight)
  = H_domain(ShardAssignment,
      epoch_seed ||
      validator_id ||
      bond_weight_be ||
      shard_count_be
    )[0..8] mod shard_count
```

Constraints:

- `shard_count > 0`
- `shard_count <= 1024`
- `validator_id` is the stable 48-byte consensus identity
- `bond_weight` is genesis/rules derived and cannot be chosen at runtime to bias
  placement
- reshuffling is driven only by the epoch seed

This keeps assignment deterministic while binding placement to an identity-cost
or stake/bond-cost input. A deployment that sets all `bond_weight = 0` is a
valid test mode but weakens shard-capture resistance.

---

## §12.3 — Shard Commitments

Each shard emits one public commitment tuple per epoch:

```
ShardCommitment = {
  shard_id:     u32,
  state_root:   [u8; 32],
  receipt_root: [u8; 32]
}
```

The EFB input list MUST contain exactly `shard_count` entries, sorted by
`shard_id`, with IDs `0..shard_count-1`. Missing, duplicate, out-of-range, or
reordered shard commitments are invalid.

Shard state execution itself remains Domain B/private. Domain A accepts only the
commitment and the verification artifacts required by the selected proof mode.

---

## §12.4 — Cross-Shard Receipts

Cross-shard communication is receipt-based and lock-free:

```
CrossShardReceipt = {
  epoch:         u64,
  source_shard:  u32,
  target_shard:  u32,
  nonce:         u64,
  payload_hash:  [u8; 32]
}
```

The receipt identifier is:

```
receipt_id = H_domain(CrossShardReceipt,
  epoch_be ||
  source_shard_be ||
  target_shard_be ||
  nonce_be ||
  payload_hash ||
  reserved_inclusion_commitment
)
```

Validity rule:

```
valid(receipt, efb) iff
  receipt.epoch == efb.epoch
  and receipt.source_shard < efb.shard_count
  and receipt.target_shard < efb.shard_count
  and receipt_id is included in the source shard receipt_root
  and efb.efb_root is the finalized EFB root for that epoch
```

The Rust scaffold enforces the epoch/shard anchor, deterministic receipt ID, and
Merkle inclusion of the receipt ID in the source shard `receipt_root`.

Replay rule:

- a receipt MUST NOT be accepted outside its EFB epoch
- `(epoch, source_shard, target_shard, nonce)` MUST be unique for a source shard
- reusing a nonce in another epoch is not replay, because the epoch is committed
  into the identifier

---

## §12.5 — Epoch Finality Beacon

The Epoch Finality Beacon is a public Domain A checkpoint:

```
EpochFinalityBeacon = {
  epoch:                  u64,
  previous_efb_root:      [u8; 32],
  shard_count:            u32,
  aggregate_state_root:   [u8; 32],
  aggregate_receipt_root: [u8; 32],
  zk_batch_root:          [u8; 32],
  efb_root:               [u8; 32]
}
```

Aggregation is deterministic over sorted shard commitments:

```
aggregate_state_root =
  fold H_domain(EpochFinalityBeacon,
    previous_aggregate || shard_id_be || state_root)

aggregate_receipt_root =
  fold H_domain(EpochFinalityBeacon,
    previous_aggregate || shard_id_be || receipt_root)
```

The final EFB root commits to:

```
epoch ||
previous_efb_root ||
shard_count ||
aggregate_state_root ||
aggregate_receipt_root ||
zk_batch_root
```

No proposer authority is required. Any node can compute the unique valid EFB
candidate from the same sorted shard commitment set. Candidates that omit,
duplicate, or reorder shard roots fail independent verification.

---

## §12.6 — ZK-STARK Verification Boundary

ZK is not a consensus shortcut and not a governance switch. It is transparent
proof compression for shard execution verification.

Modes:

- **Replay mode:** validators deterministically re-execute shard transitions.
- **STARK batch mode:** shard execution proofs are generated in Domain B and
  committed by `zk_batch_root` in the EFB.

In STARK batch mode, acceptance requires:

- transparent STARK proofs only
- proof statements bound to `(epoch, shard_id, state_root, receipt_root)`
- recursive aggregation root committed in `zk_batch_root`
- proof verifier semantics tracked in Coq/Rocq proof obligations

The provisional v1.2 ZK profile recovered from PR #93 is:

```
ZkProfile {
  engine: Plonky3,
  proof_family: FRI-STARK,
  inner_circuit_hash: Poseidon,
  outer_commitment_hash: QASH H_domain / cascade commitment surface,
  recursion_depth: 2,
  layer0: shard validity proof, no recursion,
  layer1: 16:1 recursive aggregation,
  layer2: EFB verification of aggregation proofs only
}
```

This fixes the protocol shape without introducing a production verifier. Domain
A validates only the public profile metadata and commits to `zk_batch_root`.
Domain B owns proof generation, proof-byte transport, and verifier backends.
The EFB is the only consensus path that may admit a ZK batch root. Shards may
prove asynchronously, but shard-local proving latency is not part of the
sub-50ms global commitment path.

If `zk_batch_root` is zero, the deployment is in replay mode. It must not claim
constant-time global verification.

If `zk_batch_root` is non-zero, the profile metadata must match the fixed
Plonky3/FRI/Poseidon/QASH profile above. Other engines, deeper recursion, KZG
SNARKs, Bulletproofs, or alternate aggregation factors are separate protocol
extensions and must not be accepted under this profile ID.

---

## §12.7 — Public Transcript

The public observation surface becomes:

```
PublicTranscript(epoch_t) = {
  state_root_t,
  receipt_root_t,
  efb_root_t,
  epoch_t,
  halt_flag_t
}
```

`efb_root_t` is safe to publish because it is a fixed-length commitment over
public roots. It does not expose raw transactions, receipt leaves, validator
edge topology, admission transport, sender, receiver, amount, or action type.

---

## §12.8 — Implementation Mapping

Rust:

- `crates/consensus/src/sharding.rs`
  - `assign_shard`
  - `CrossShardReceipt`
  - `receipt_id`
  - `receipt_is_epoch_anchored`
  - `verify_receipt_inclusion`
  - `ShardCommitment`
  - `EpochFinalityBeacon`
  - `compute_efb`
- `crates/consensus/src/public.rs`
  - adds `efb_root` to `PublicTranscript`
- `crates/consensus/src/envelope.rs`
  - v1.2 envelope includes explicit `shard_id`
- `crates/consensus/src/transition.rs`
  - `advance_epoch_sharded` computes the EFB during the epoch commit path
  - `TransitionResult.public_transcript` publishes `(state_root, receipt_root, efb_root)`
  - v1.2 state roots commit to the aggregate receipt root and EFB root
- `tests/vectors/vectors.v1.2.json`
  - pins multi-epoch sharded replay roots for cross-ISA verification

Proof tracking:

- `proofs/sharding/efb_determinism.v`
  - EFB aggregation is deterministic for identical inputs
  - receipt epoch binding rejects cross-epoch replay

Deferred implementation work:

- STARK verifier/aggregator feature gate
- Plonky3-backed Domain B verifier implementing the fixed profile
- Poseidon circuit transcript with QASH-native outer commitment binding
- 2-layer recursion corpus: Layer 0 shard proofs, Layer 1 16:1 aggregation,
  Layer 2 EFB batch-root verification
- adversarial shard-capture simulation with configured bond weights
