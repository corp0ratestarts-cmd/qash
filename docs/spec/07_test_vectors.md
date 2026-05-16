# QASH Canonical Test Vectors

**Status:** Normative  
**Backs theorem:** TH-7 (replay invariance across all authorized ISAs)

---

## Purpose

These vectors define the expected outputs for specific, fully-specified input
sequences.  Any implementation claiming QASH compatibility MUST reproduce every
value below on every authorized ISA (x86\_64, aarch64, riscv64gc).  A mismatch
is a protocol violation, not a platform quirk.

Each test case maps to a named Rust test.  CI runs those tests on all three
ISA targets.

---

## Notation

- All byte sequences: `[b0, b1, ..., bN]` in decimal, little-endian where indicated.
- `genesis_state`: `epoch=0`, `halt=None`, `entropy_seed=[0u8;32]`,
  `validator_count=4`, all `ValidatorMetrics::ZERO`, `nonces=[0;1024]`,
  `validator_ids=[[0u8;48];1024]`, `state_root=[0u8;32]`.
- `idle_input(N)`: `update_count=N`, all `updates[i]=None`.
- `TX0(author_id, nonce)`: wire bytes for a TX-0 (nonce-advance) envelope with
  the given author and nonce.  See `§4 wire format` in `03_transactions.md`.
- Fixed-point values use scale `1_000_000` (see `GENESIS_CONSTANTS.toml`).

---

## Test Vectors

### TV-0 — Genesis state root

**Input:** `genesis_state`  
**Expected:** `state_root = [0u8; 32]`  
**Rust test:** `state_root_genesis_is_zero` (`crates/consensus/src/transition.rs`)

The genesis state carries `state_root=[0u8;32]` before any epoch is advanced.

---

### TV-1 — 3-epoch idle root (TH-7 anchor)

**Input:** `genesis_state` → 3 × `advance_epoch(idle_input(4), [])`  
**Expected state_root after epoch 3:**

```
[138, 219, 164, 211,  10,  54,  30,  39,
 151, 223, 239,  42, 191, 141,  13, 181,
 121, 224,  79, 241,   4,  74,  49,  44,
 138, 224,  93, 197, 103, 104, 122, 198]
```

**Rust test:** `state_root_canonical_seq_golden` (`crates/consensus/tests/golden_replay.rs`)

This is the primary TH-7 cross-ISA anchor.  The value is reproduced by the
`verify_two_stage_build.sh` script on all three ISA targets.

---

### TV-2 — TX-0 nonce advance

**Input:** `genesis_state` with `validator_ids[0][0]=1`, `validator_ids[1][0]=2`;
then `advance_epoch(idle_input(4), [TX0(validator_ids[0], 0), TX0(validator_ids[1], 0)])`

**Expected:** `nonces[0]=1`, `nonces[1]=1`, `nonces[2]=0`, `nonces[3]=0`

**Rust test:** `full_epoch_with_tx0` (`crates/consensus/tests/golden_replay.rs`)

---

### TV-3 — TX-0 replay rejection

**Input:** same as TV-2, then a second `advance_epoch` with `TX0(validator_ids[0], 0)`
(nonce already consumed).

**Expected:** `nonces[0]` unchanged at `1` after epoch 2 (duplicate TX ignored, not halted).

**Rust test:** `tx0_replay_rejected` (`crates/consensus/src/transaction.rs`)

---

### TV-4 — TX-0 unknown author rejection

**Input:** `genesis_state` (all `validator_ids=[0u8;48]`);
`advance_epoch(idle_input(4), [TX0([0xABu8; 48], 0)])` — author not in validator set.

**Expected:** `nonces` unchanged; epoch advances normally (TX ignored).

**Rust test:** `tx0_unknown_author_ignored` (`crates/consensus/src/transaction.rs`)

---

### TV-5 — Halt trigger (H1: Lyapunov violation)

**Input:** `genesis_state` → 3 × `idle_input(4)` (fills window, `V_convergence=0` each);
then `advance_epoch(spike_input, [])` where `spike_input.updates[0].divergence_new = 1_000_000`
(= `SCALE`), all others `None`.

**Expected:** `Err(HaltReason::LyapunovViolation)`.

**Rust test:** `halt_freezes_entire_state_except_halt_reason`
(`crates/consensus/tests/golden_replay.rs`)

---

### TV-6 — Halt is absorbing

**Input:** TV-5 halted state; 10 subsequent calls to `advance_epoch`.

**Expected:** every call returns `Err(HaltReason::LyapunovViolation)`;
state fingerprint is identical before and after every call.

**Rust test:** `golden_halt_reason_preserved`
(`crates/consensus/tests/golden_replay.rs`)

---

### TV-7 — Entropy chain determinism

**Input:** `genesis_state` → N × `idle_input(4)` for any N ≥ 1.

**Expected:** `entropy_seed ≠ [0u8;32]` after epoch 1; identical value across
x86\_64, aarch64, riscv64gc for the same N.

Derivation formula: `entropy_seed_{t+1} = h_domain(EntropyAdvance, entropy_seed_t)`
where `h_domain(tag, data) = SHA3-256(tag_u32_le || data)`.

**Rust test:** `entropy_seed_advances_nonzero`
(`crates/consensus/src/transition.rs`)

---

### TV-8 — Hash cascade determinism

**Input:** any fixed 32-byte buffer `b`.

**Expected:** two independent calls to `h_domain(tag, b)` with the same tag
produce byte-identical outputs on the same ISA and across all authorized ISAs.

Scope: the current Domain A cascade implementation (SHA3-256 with domain
separation via tag prefix).  BLAKE3 and KangarooTwelve cascade stages are
deferred to the Domain B specification; their determinism test vectors will be
added in a future revision of this document.

**Rust test:** `cascade_determinism_same_input`
(`crates/consensus/src/hash.rs`)

---

## Gate Rule (TH-7)

A build is considered TH-7-passing when:

1. `cargo test -p qash-consensus --no-default-features` exits 0 on **all three** of
   `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `riscv64gc-unknown-linux-gnu`.
2. The `state_root_canonical_seq_golden` test produces the exact TV-1 byte sequence
   on each target.
3. The `./scripts/verify_two_stage_build.sh` script exits 0.

A genesis proposal that lacks TH-7 CI evidence for all three ISA targets is inadmissible.

---

## Regeneration Procedure

If a deliberate protocol change alters TV-1 (e.g., encoding format change), the
new canonical root MUST be regenerated as follows:

```sh
PRINT_GOLDEN=1 cargo test -p qash-consensus --no-default-features \
    -- --nocapture state_root_canonical_seq_print
```

Update `EXPECTED_STATE_ROOT_3_EPOCHS` in `golden_replay.rs` ONLY after verifying
that all three ISA targets produce the new value and that the change is intentional.

Any change to TV-1 constitutes a fork.
