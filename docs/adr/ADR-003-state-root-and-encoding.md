# ADR-003 — Define `compute_state_root(state, crypto_suite)` encoding + commitment

**Status:** Accepted  
**Filed:** 2026-05-13  
**Revised:** 2026-05-30 (full byte-layout specification added; Phase 1-C)  
**PDF anchor:** §4.2 (p. 10) calls `compute_state_root(state, crypto_suite)` but does not define encoding.  
**Supersedes:** `ADR-003-state-root-encoding.md` (earlier draft; status: SUPERSEDED).

---

## Decision

Define:

1. **Canonical `Encode_for_commitment(State, prior_root)`** — deterministic little-endian byte layout
   with explicit field ordering, padding rules, bounds, and rejection criteria.
2. **`compute_state_root(state, prior_root)`** — applies `H_domain(StateRoot, preimage)` to the
   canonical encoding.

**Constraint:** `H_cascade` / `cascade::h_cascade` is **not** the v1.0 state-root commitment and
MUST NOT be substituted, truncated, folded, or otherwise adapted for genesis state roots. Activating
cascade-derived state roots requires a post-genesis migration ADR with an explicit
commitment/truncation rule and new KAT vectors.

---

## Byte-Layout Specification

### 1. `H_domain` tag function

```
H_domain(tag: u32, input: &[u8]) -> [u8; 32]
  = SHA3-256( tag_u32_le || input )
```

Domain tag for state roots: `DomainTag::StateRoot = 0x0000_0001`.

Encoding: `tag` is written as a 4-byte little-endian prefix before `input` is fed into SHA3-256.
This is a one-shot call; no length prefix is added beyond the tag.

### 2. State-root commitment preimage

The preimage is the concatenation of the sections below in order. All multi-byte integer fields use
**little-endian** encoding. Padding bytes are always `0x00`.

#### 2.1 Fixed header (120 bytes)

| Offset | Size | Type    | Field            | Notes |
|--------|------|---------|------------------|-------|
| 0      | 8    | u64 LE  | `epoch`          | Current epoch number |
| 8      | 32   | bytes   | `prior_root`     | **Not** `state.state_root`. The prior epoch's committed root, passed in as `prior_root` argument. Binds the commitment chain. |
| 40     | 32   | bytes   | `ledger_root`    | All zeros in v1.0 (no ledger yet). Must be exactly `[0u8; 32]`. |
| 72     | 32   | bytes   | `entropy_seed`   | Epoch entropy seed |
| 104    | 1    | u8      | `halt_reason`    | `HaltReason` discriminant byte |
| 105    | 3    | pad     | —                | `[0x00, 0x00, 0x00]` (canonical padding; rejection on decode if non-zero) |
| 108    | 4    | u32 LE  | `validator_count`| Number of active validators (≤ MAX_VALIDATORS = 1024) |
| 112    | 4    | u32 LE  | `cascade_health` | v1.1 cascade health counter (introduced alongside streaming encoding in ADR-012) |
| 116    | 4    | pad     | —                | `[0x00, 0x00, 0x00, 0x00]` |

Total: 120 bytes.

#### 2.2 Validator slots (80 bytes × `validator_count`)

For each validator slot `i` in `0..validator_count`, in slot order:

| Offset (within slot) | Size | Type   | Field                       | Notes |
|----------------------|------|--------|-----------------------------|-------|
| 0                    | 8    | i64 LE | `divergence` (wire)         | `FixedPoint.raw()` cast to i64; valid range `[0, SCALE]` = `[0, 1_000_000]` |
| 8                    | 8    | i64 LE | `conflict` (wire)           | `FixedPoint.raw()` cast to i64; valid range `[0, SCALE]` |
| 16                   | 8    | i64 LE | `slash_accum` (wire)        | `FixedPoint.raw()` cast to i64; valid range `[0, i64::MAX]` |
| 24                   | 8    | u64 LE | `nonce[i]`                  | TX replay nonce for slot `i` |
| 32                   | 48   | bytes  | `validator_ids[i]`          | Fixed 48-byte consensus identity |

Total per slot: 80 bytes.

**Wire encoding of FixedPoint:** The raw `i128` is cast to `i64` via `fp_to_i64_wire()`.
All production consensus values satisfy this invariant before reaching the commit phase:
- `divergence`, `conflict` ∈ `[0, 1_000_000]` (fits i64 trivially).
- `slash_accum` ∈ `[0, i64::MAX]` (enforced by `advance_epoch` at transition line ~459).

#### 2.3 Convergence window (28 bytes)

| Offset (within section) | Size | Type   | Field        | Notes |
|-------------------------|------|--------|--------------|-------|
| 0                       | 1    | u8     | `filled`     | Number of filled window slots (0–3) |
| 1                       | 3    | pad    | —            | `[0x00, 0x00, 0x00]` |
| 4                       | 8    | i64 LE | `values[0]`  | FixedPoint raw as i64 |
| 12                      | 8    | i64 LE | `values[1]`  | |
| 20                      | 8    | i64 LE | `values[2]`  | |

Total: 28 bytes.

#### 2.4 Sharding extension (0 or 64 bytes) — v1.2+

**Condition:** appended only if `receipt_root ≠ [0u8; 32]` OR `efb_root ≠ [0u8; 32]`.
In v1.0 genesis, both fields are zero; this section is absent.

| Offset (within section) | Size | Type  | Field          |
|-------------------------|------|-------|----------------|
| 0                       | 32   | bytes | `receipt_root` |
| 32                      | 32   | bytes | `efb_root`     |

### 3. Commitment computation

```
preimage = Encode_for_commitment(state, prior_root)
state_root = H_domain(0x0000_0001, preimage)
           = SHA3-256( [0x01, 0x00, 0x00, 0x00] || preimage )
```

Output: 32 bytes.

### 4. Total preimage size

```
min_size = 120 (header) + 80 × validator_count + 28 (window)
max_size (with sharding) = min_size + 64
```

For `validator_count = 1024` with sharding:
`120 + 80 × 1024 + 28 + 64 = 120 + 81920 + 28 + 64 = 82132 bytes`

Constant `MAX_COMMITMENT_PREIMAGE = 24717` in the codebase refers to the encoding helper in
`crates/consensus/src/encoding.rs`, which covers a subset of state. The full streaming preimage
(via `stream_state_for_commitment`) does not require a pre-allocated buffer; see ADR-012.

### 5. Canonical rejection rules (decode)

A decoded state encoding is rejected (returns `EncodeError::DecodeInvalid`) if:

- `version != ENCODING_VERSION` (i.e., not 0)
- Padding bytes at offsets 105–107 are non-zero
- Padding bytes at offsets 116–119 are non-zero
- `halt_reason` byte has no known variant
- `divergence.raw()` or `conflict.raw()` is negative or > `SCALE` (1_000_000)
- `slash_accum.raw()` is negative or > `i64::MAX`
- `validator_count > MAX_VALIDATORS` (1024)

### 6. Field not included in commitment

`causal_fingerprint` (`[u8; 32]`) is **not** included in the state-root commitment preimage. It is
maintained as a parallel causal-history chain for divergence detection (`fp = H_domain(CausalFingerprint, prev_fp || epoch_le || state_root)`). Equal fingerprints imply bisimilar states; see `proofs/safety/causal_fingerprint.v`. Its exclusion from the state root is intentional: fingerprint serves as a separate audit trail, not a commitment.

---

## Implementation

| Function | Location | Purpose |
|----------|----------|---------|
| `compute_state_root(state, prior_root)` | `crates/consensus/src/transition.rs:448` | Reference impl (test-only); calls `stream_state_for_commitment` |
| `ProjectedView::compute_root(prior_root)` | `crates/consensus/src/transition.rs:492` | Production path; same byte sequence, no full-state allocation |
| `stream_state_for_commitment(state, prior_root, h)` | `crates/consensus/src/transition.rs:412` | Test-only streaming path |
| `encode_full_state_into(state, out)` | `crates/consensus/src/transition.rs:144` | Full buffer encode (for snapshot/replay) |
| `encode_state_header(...)` | `crates/consensus/src/encoding.rs:20` | Header section encoding |
| `decode_state_header(bytes)` | `crates/consensus/src/encoding.rs:37` | Header decode with rejection rules |
| `encode_validator_dynamic(...)` | `crates/consensus/src/encoding.rs:93` | Per-validator 48-byte commitment encoding (i128 form) |
| `decode_validator_dynamic(bytes)` | `crates/consensus/src/encoding.rs:111` | Per-validator decode with bounds check |
| `h_domain(tag, input)` | `crates/consensus/src/hash.rs:35` | Tag-prefixed SHA3-256 |
| `h_domain_start(tag)` / `h_domain_finish(h)` | `crates/consensus/src/hash.rs:48/55` | Streaming SHA3-256 with domain prefix |

---

## Acceptance Criteria

- [x] Golden vectors with expected `state_root_hex` and a genesis state-root commitment KAT in `tests/vectors/vectors.v1.json`
- [x] Roundtrip: `Decode(Encode(S)) == S` for valid states
- [x] Canonical rejection tests (non-canonical encodings fail)
- [x] `ProjectedView::compute_root` produces byte-for-byte identical output to `compute_state_root` on identical state (enforced by `projected_view_compute_root_matches_full_state` test)
- [x] Multi-compiler differential test (opt-level 0 vs 3, Cranelift) confirms identical state roots
- [ ] PDF-golden vectors: citations to PDF §4.2 must be manually verified against `spec/pdf/QASH_Spec_v1.0.pdf` (Phase 1-D — human review task)

---

## Traceability

| Property | Traceability row | Status |
|----------|-----------------|--------|
| State root binding | P0-2 | Partial — code-derived vectors pass; PDF-golden pending 1-D |
| Canonical encoding determinism | P0-1 | ✅ Multi-compiler CI |
| Canonical rejection | P0-5 | ✅ Unit tests |
