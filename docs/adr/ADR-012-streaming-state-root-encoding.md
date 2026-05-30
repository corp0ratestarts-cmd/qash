# ADR-012: Streaming State-Root Canonical Encoding

**Status:** Accepted
**Date:** 2026-05-29
**Source:** Phase 2-R runtime optimization track (ADR-006); genesis-lock audit
**PDF authority:** PDF-SILENT (implementation strategy, not ontology)

## Context

`docs/spec/01_consensus.md §3` and `docs/spec/00_execution_model.md §E4`
define `Encode_for_commitment(S)` as a deterministic byte sequence — the
canonical preimage fed to SHA3-256 when computing the state root. The spec
defines the **ontology** (the exact byte layout and field ordering) but is
intentionally silent on whether implementations must materialise the full
preimage as a contiguous buffer before hashing.

The current implementation in `crates/consensus/src/encoding.rs` constructs
an ~82 KB stack/heap buffer, writes all canonical fields into it, then passes
the whole buffer to `SHA3-256`. This is correct but has measurable memory
pressure on constrained validators.

An auditor reviewing Phase 2-R might flag a streaming implementation as a
spec deviation. This ADR removes that ambiguity.

## Decision

Implementations **MAY** stream canonical fields directly into the SHA3-256
sponge without materialising a contiguous intermediate buffer, provided:

1. **Byte-identical preimage:** The exact sequence of bytes absorbed by the
   sponge is bitwise identical to `Encode_for_commitment(S)` as defined in
   `docs/spec/00_execution_model.md §E4`. Field order, length prefixes,
   domain tags, and padding are unchanged.

2. **Golden-vector parity:** The streaming path must produce the same
   state-root output as the buffered path on every test vector in
   `tests/vectors/vectors.v1.json` and the `golden_replay` corpus.

3. **Cross-ISA parity:** Streaming and buffered paths must produce identical
   state roots on x86_64, aarch64, and riscv64gc (enforced by the
   `platform-determinism` CI job and `verify_two_stage_build.sh`).

4. **Domain A constraints satisfied:** The streaming encoder remains in
   Domain A — no `unsafe`, no floats, no entropy ingress, checked arithmetic.

## Rationale

The spec's canonical encoding ontology defines the exact *byte sequence*,
not the *memory layout* used to produce it. Streaming a deterministic byte
sequence into a hash primitive is observationally equivalent to buffering it
as long as the absorbed bytes are identical. This is the same principle that
allows Merkle trees to be computed incrementally rather than by materialising
the full leaf array.

## Constraints

- This ADR does **not** authorise changing the canonical field ordering,
  length prefixes, or domain tags.
- Streaming optimisations must not be interleaved with nondeterministic
  operations or I/O.
- The buffered reference path in `crates/consensus/src/encoding.rs` must
  remain present (possibly behind a feature flag) as the parity oracle for
  tests until the streaming path accumulates sufficient CI evidence.

## Evidence Required Before Streaming Path Is Default

- [ ] Preimage equivalence tests comparing buffered vs. streaming output on
      all `vectors.v1.json` test cases.
- [ ] `golden_replay` regression suite passes on streaming path.
- [ ] Criterion benchmark showing ≥15% reduction in state-root latency at
      n=1024 validators.
- [ ] Cross-ISA CI (`platform-determinism`) green on streaming path.

## Consequences

Phase 2-R may implement and land the streaming state-root path without
spec change, provided the evidence requirements above are met. Auditors
reviewing the genesis-lock evidence package should treat the streaming
implementation as a compliant optimisation of the canonical encoding
ontology, not a deviation from it.
