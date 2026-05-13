# ADR-003 — Define `compute_state_root(state, crypto_suite)` encoding + commitment
**Status:** Proposed  
**Filed:** 2026-05-13  
**PDF anchor:** §4.2 (p. 10) calls `compute_state_root(state, crypto_suite)` but does not define encoding.

## Decision
Define:
- Canonical `Encode(State)` (byte layout + bounds + rejection rules)
- `Encode_for_commitment(State, prior_root)` (if prior-root binding is required)
- `state_root = H_domain(STATE_ROOT, Encode_for_commitment(...))`

## Acceptance criteria
- Roundtrip: `Decode(Encode(S)) == S` for valid states
- Canonical rejection tests (non-canonical encodings fail)
- Golden vectors with expected `state_root_hex`
