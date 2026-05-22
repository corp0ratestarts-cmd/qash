# ADR-007: UC-MJA Cascade Research Track

**Status:** Proposed
**Date:** 2026-05-22
**Source:** PR #93 multi-jurisdictional cascade review
**PDF authority:** PDF-SILENT

## Context

The latest PR #93 cryptographic-design comment proposes a multi-jurisdictional
anchor that combines sovereign hash standards, an XOF binding layer,
information-theoretic message authentication, and zero-knowledge compliance
proofs. The review also identifies three designs that are incompatible with the
QASH consensus kernel:

- GF(2^512) polynomial MAC in Domain A, because it is too expensive to justify
  against the existing sub-2ms envelope target.
- Argon2id or any memory-hard function in Domain A, because Domain A is
  `no_alloc`, fixed-size, replay-deterministic code.
- STARK proving or per-envelope ZK work in Domain A, because proof generation is
  outside the consensus hot path and depends on heavy Domain B resources.

## Decision

Schedule a dedicated UC-MJA research and implementation track. The track may
define a future cascade-backed compliance anchor only if it preserves the
existing Domain A invariants and keeps heavy cryptographic proof work in Domain
B.

Accepted design constraints:

- Use fixed-size Domain A arrays only.
- Bind the cascade paths through SHAKE256 or another explicitly specified XOF
  with fixed output lengths and deterministic domain separators.
- Treat SM3 as a 256-bit primitive only; any double-width SM3 construction must
  be domain separated, for example `SM3(0x01 || input) || SM3(0x02 || input)`.
- If a polynomial MAC is added to Domain A, cap it at GF(2^256) unless local
  benchmarks and proof obligations justify a larger field.
- Keep ZK proof generation, proof-byte transport, and verifier backend
  integration in Domain B or the sharding aggregation layer.
- Require formal proof obligations before any consensus commitment change:
  XOF binding, MAC unforgeability, constant-time behavior, and cross-ISA replay
  invariance.

Rejected designs:

- No GF(2^512) MAC as a default Domain A hot-path primitive.
- No Argon2id, scrypt, or other memory-hard function in Domain A.
- No per-transaction or per-epoch STARK proving in Domain A.
- No hardware-specific intrinsics that can change authorized-ISA replay
  behavior.
- No replacement of the v1.0 SHA3-256 state root without a separate genesis or
  network-definition decision.

## Required Evidence

Before UC-MJA can move beyond a research track, it must provide:

- a normative spec update for path order, domain separators, XOF output layout,
  MAC input blocks, and public transcript binding
- fixed KAT vectors for SHA3, double-width SM3, Streebog, Kupyna, LSH, BLAKE3
  XOF, KangarooTwelve, and Skein paths when those paths are enabled
- Criterion benchmarks showing the Domain A anchor remains within the published
  performance targets
- cross-ISA parity on x86_64, aarch64, and riscv64gc
- constant-time audit evidence for MAC arithmetic
- Coq/Rocq obligations for XOF binding assumptions and MAC unforgeability
- Domain B verifier KATs for any ZK-QASH compliance proof backend

## Consequences

UC-MJA becomes a scheduled cryptographic research track rather than an implicit
change to `H_domain` or the existing `H_cascade` spec. This lets QASH explore
multi-jurisdictional compliance anchors while preserving the current consensus
byte contract and avoiding memory-hard or prover-heavy work in Domain A.
