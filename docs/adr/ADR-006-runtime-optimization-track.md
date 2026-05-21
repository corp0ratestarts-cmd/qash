# ADR-006: Runtime Optimization Track

**Status:** Proposed
**Date:** 2026-05-21
**Source:** PR #93 runtime-performance review
**PDF authority:** PDF-SILENT

## Context

The latest PR #93 review identifies consensus hot-path inefficiencies that can
be improved without changing protocol semantics: duplicate transaction parsing,
quadratic candidate ordering pressure, buffered state-root construction, and
full-state projection copies. The review also warns that performance work must
not alter consensus bytes, hash preimages, wire formats, or Domain A
determinism.

## Decision

Schedule a dedicated `Phase 2-R: Core Runtime Optimization` track. The track is
allowed to refactor runtime data movement and algorithms only when every change
preserves byte-identical consensus outputs.

Accepted work items:
- Single-pass transaction admission with deterministic candidate records.
- Deterministic total-order sorting by `(sort_key, tx_id)`.
- Streaming state-root hashing with exact canonical preimage parity.
- Runtime-only `ProjectedView` to reduce full `EpochState` copies.
- Optional validator-directory sidecar if profiling justifies it.

Rejected in this track:
- Wire-format changes.
- State-root preimage changes.
- Public API semantics changes.
- New transaction semantics.
- ZK, sharding, PAL networking, or proof-system work.

## Constraints

All work remains inside Domain A rules:
- no `unsafe`
- no floats
- no `HashMap` or nondeterministic iteration
- no wall clock or entropy ingress
- checked arithmetic only
- no persisted runtime sidecars in consensus state

The Coq model continues to describe logical state transitions. Runtime-only
views or streaming encoders must be proven by tests to be observationally
equivalent to the existing buffered/logical path.

## Required Evidence

Before any Phase 2-R implementation can support a performance or genesis-lock
claim, it must provide:
- golden/vector replay parity
- cross-ISA state-root parity on x86_64, aarch64, and riscv64gc
- total-order tests for identical sort keys and reversed input batches
- preimage equivalence tests for streaming state-root commitment
- Criterion benchmark groups for tx-heavy admission, sorting, validator lookup,
  state-root commitment, and view-based epoch advancement
- archived benchmark artifacts under `artifacts/benchmarks/`

Initial precondition coverage is present in
`crates/consensus/tests/phase2r_preconditions.rs` and
`crates/consensus/benches/epoch_transition.rs`. These gates establish parity
and benchmark-compilation surfaces; they do not by themselves implement the
runtime optimization track or authorize performance claims.

## Consequences

This creates a clear landing zone for the PR #93 runtime-performance feedback
without mixing it into the sharding/ZK detour. It also prevents performance
refactors from silently weakening TH-7 replay invariance or the Domain A proof
eligibility constraints.
