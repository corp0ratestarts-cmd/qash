# ADR-003: Full State Encoding and State Root Commitment

- **Status:** proposed
- **PDF anchor:** §4.2, p. 10
- **Traceability rows:** P0-2, P0-9

## Verbatim PDF text

```text
let new_state = compute_state_root(state, crypto_suite);
```

## Context

The PDF calls `compute_state_root(state, crypto_suite)` but does not define the
canonical bytes of `state`, the serialization format, or whether the root
commits to the full state or a summary.

## Decision

Superseded by accepted `ADR-003-state-root-and-encoding.md`: v1.0 genesis state roots use `H_domain(STATE_ROOT, Encode_for_commitment(...))` (SHA3-256 over `tag_u32_le || input`). `H_cascade` is not active for v1.0 state roots; any cascade state-root activation is a post-genesis migration item requiring a separate ADR and fresh KAT vectors.

Define a full canonical state encoding before genesis lock. The definition must
include:

- every state field included in the commitment,
- field order,
- integer endianness,
- validator array encoding,
- Lyapunov window encoding,
- canonical rejection rules,
- versioning, if any,
- the exact input bytes to domain-separated state-root hashing.

## Consequences

`state_root_header_only()` is a provisional implementation detail. It cannot be
treated as genesis-ready until this ADR is accepted and golden vectors lock the
resulting bytes.
