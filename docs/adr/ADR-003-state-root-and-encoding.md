# ADR-003 — Define `compute_state_root(state, crypto_suite)` encoding + commitment
**Status:** Accepted  
**Filed:** 2026-05-13  
**PDF anchor:** §4.2 (p. 10) calls `compute_state_root(state, crypto_suite)` but does not define encoding.

## Decision
Define:
- Canonical `Encode(State)` (byte layout + bounds + rejection rules)
- `Encode_for_commitment(State, prior_root)` (if prior-root binding is required)
- For v1.0 genesis, `state_root = H_domain(STATE_ROOT, Encode_for_commitment(...))`, where `H_domain` is SHA3-256 over `tag_u32_le || input`.

`H_cascade` / `cascade::h_cascade` is **not** the v1.0 state-root commitment and MUST NOT be substituted, truncated, folded, or otherwise adapted for genesis state roots. Activating cascade-derived state roots requires a post-genesis migration ADR with an explicit commitment/truncation rule and new KAT vectors.

## Implementation
- `encode_commitment_preimage()` in `crates/consensus/src/encoding.rs` — prior-root substitution, deterministic LE layout, MAX_COMMITMENT_PREIMAGE = 24717 bytes
- `compute_state_root()` in `crates/consensus/src/transition.rs` — `H_domain(STATE_ROOT, preimage)` / SHA3-256
- `EpochState.state_root` and `EpochState.ledger_root` fields track commitment chain

## Acceptance criteria
- [x] Golden vectors with expected `state_root_hex` and a genesis state-root commitment KAT in `tests/vectors/vectors.v1.json`
- [ ] Roundtrip: `Decode(Encode(S)) == S` for valid states — pending dedicated roundtrip tests
- [x] Canonical rejection tests (non-canonical encodings fail) — covered in `crates/consensus/tests/domain_a_audit.rs` (`encoding_audit` module).
