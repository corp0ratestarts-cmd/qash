# QASH Constitutional Scarcity Axiom

**Status:** Normative  
**Scope:** Pure QASH monetary policy — Domain A economic surface.

---

## §T0 — Monetary standard

Pure QASH uses a single-token monetary standard with genesis-locked constants.
There is no oracle, rebase, peg, bonding curve, dual token, or discretionary treasury.
There is no monetary governance: no on-chain vote can change any monetary parameter.

---

## §T1 — Core economic law

All values are integers. No floating-point arithmetic anywhere in this spec.

```
reward(epoch) =
  max(TAIL_REWARD, INITIAL_REWARD >> floor(epoch / DECAY_INTERVAL))

required_fee(tx) =
  deterministic_resource_cost(tx)       # no bidding; pure resource cost

attached_fee(tx) must equal required_fee(tx)   # exact equality; over/under = reject

fee_burn(tx) = required_fee(tx)         # fee_burn_policy = "total"

slash_burn(amount) = amount             # slash_burn_policy = "total"

validator_reward_pool(epoch) = reward(epoch)   # no fee revenue for validators

new_total_supply =
  old_total_supply
  + reward(epoch)
  - burned_fees
  - burned_slashes
```

---

## §T2 — Parameter definitions

All parameters come from `GENESIS_CONSTANTS.toml [economics]`. They are genesis-locked.

| Parameter | TOML key | Notes |
|---|---|---|
| `INITIAL_REWARD` | `initial_reward_atomic` | Starting per-epoch issuance |
| `DECAY_INTERVAL` | `decay_interval_epochs` | Halving period (~60 months) |
| `TAIL_REWARD` | `tail_reward_atomic` | Fixed nominal floor; never zero |
| Fee burn policy | `fee_burn_policy = "total"` | 100% of fees burned; no BPS math |
| Slash burn policy | `slash_burn_policy = "total"` | 100% of slashes burned |

---

## §T3 — MEV-null economic surface (§A12)

```
1.  Fees are deterministic resource costs, not bids.
2.  Attached fee must exactly equal required_fee(tx). Deviation → reject.
3.  All fees are burned. fee_burn_policy = "total".
4.  Validators receive no transaction-fee revenue.
5.  No priority fee exists.
6.  No transaction ordering may depend on fee excess.
7.  Non-conflicting economic transactions must commute.
8.  Conflicting spends (shared nullifiers) are annihilated: all TXs in the
    conflict class are rejected before application (not FIFO first-wins).
9.  No AMM, order book, liquidation, lending, auction, oracle-price collateral,
    bridge-price, or programmable callback may exist in Domain A.
10. Any transaction type violating the above is inadmissible in Pure QASH.
```

---

## §T4 — OrderImage and sort key

To prevent signature-grinding attacks on ordering:

```
OrderImage(τ) = canonical transaction body excluding authorization/signature/proof bytes
sort_key(τ, S_t) = H_domain(ENTROPY_ADVANCE, S_t.entropy_seed || OrderImage(τ))
```

Per-type OrderImage:
- TX-0/TX-1 (consensus heartbeat types): `version || tx_type || nonce || author_id || payload_len || payload`
- PTX-0 (Pure cash transfer): `tx_type || validated_nullifiers_root || output_commitments_root || fee_amount_le16 || epoch_domain`

TxID may still use full `Encode(τ)` (including signature) for replay identity.

---

## §T5 — EconomicsState

`EconomicsState` is a Domain A struct included in canonical `EpochState` encoding.
It is state-rooted — any change to supply is reflected in `state_root`.

```rust
pub struct EconomicsState {
    pub total_supply:         Amount,   // current circulating supply
    pub issued_total:         Amount,   // cumulative issued (monotone increasing)
    pub burned_fees_total:    Amount,   // cumulative burned fees
    pub burned_slashes_total: Amount,   // cumulative burned slashes
}
```

Conservation invariant: `total_supply = issued_total - burned_fees_total - burned_slashes_total`

---

## §T6 — Formal theorem targets

All theorems below start as `TARGET` (Admitted stubs in Coq). No theorem is marked
`PROVED` until CI compiles actual proof content.

| Theorem | Description | Status |
|---------|-------------|--------|
| TH-E1 | Supply Delta Determinism | TARGET |
| TH-E2 | Mint Confinement | TARGET |
| TH-E3 | Reward Monotonicity | TARGET |
| TH-E4 | Tail Boundedness | TARGET |
| TH-E5 | Burn Irreversibility | TARGET |
| TH-E6 | Supply Arithmetic Safety | TARGET |
| TH-E7 | Oracle Non-Interference | TARGET |
| TH-E8 | Parameter Immutability | TARGET |
| TH-E9 | Fee Ordering Non-Interference | TARGET |
| TH-E10 | Economic Commutativity | TARGET |
| TH-E11 | Conflict Annihilation | TARGET |
| TH-E12 | Signature Ordering Non-Interference | TARGET |
| TH-E13 | Inclusion Completeness | TARGET |
| TH-E14 | No Application-Layer MEV Surface | TARGET |

---

## §T7 — Acceptance checklist

- [ ] No priority fee anywhere in Domain A
- [ ] No base-fee/tip split
- [ ] No floating-point percentages
- [ ] No oracle-based supply adjustment
- [ ] No governance-adjustable monetary parameter
- [ ] All fees burn (policy = "total")
- [ ] All slashes burn (policy = "total")
- [ ] Validators receive no fee revenue
- [ ] Economics constants are all integers in GENESIS_CONSTANTS.toml
