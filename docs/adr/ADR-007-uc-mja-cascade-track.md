# ADR-007: UC-MJA Cascade Research Track

**Status:** Proposed  
**Date:** 2026-05-24  
**Source:** PR #93 follow-through reconciliation

## Context

PR #93 raised a multi-jurisdictional cascade proposal involving sovereign hash
paths, XOF binding, polynomial MACs, and ZK compliance proofs. The idea may be
useful, but it must not enter Domain A as an unconstrained performance or
compliance shortcut.

QASH's current priority remains pre-genesis evidence closure, zero-persistence
Domain B implementation, and production PAL hardening.

## Decision

Schedule UC-MJA as a research track only. It is not a genesis blocker and not a
consensus feature until separate specs, KATs, benchmarks, constant-time evidence,
Coq/Rocq obligations, and cross-ISA parity exist.

Accepted research scope:

- fixed-size Domain A array outputs only;
- SHAKE256 or equivalent XOF binding with fixed output layout;
- sovereign hash paths only through explicit domain separators;
- compact polynomial MAC exploration only if constant-time and benchmarked;
- ZK compliance proofs only in Domain B or shard aggregation;
- Domain A may commit to public roots but must not ingest prover state.

Rejected scope:

- GF(2^512) or memory-hard MACs in the Domain A hot path;
- Argon2id, scrypt, or memory-hard cross-binding in Domain A;
- per-envelope or per-epoch STARK proving in Domain A;
- hardware-specific intrinsics that weaken cross-ISA determinism;
- replacing current hash-domain commitments without a separate network-definition decision.

## Required evidence before implementation

- Normative spec delta for domain separators and output layout.
- KAT corpus for every enabled path.
- Criterion benchmarks against the published performance target envelope.
- Cross-ISA parity on x86_64, aarch64, and riscv64gc.
- Constant-time audit evidence for any arithmetic added to Domain A.
- Formal obligations filed in `proofs/COVERAGE.md`.
- Domain B verifier KATs for any ZK compliance backend.

## Consequences

This keeps the idea alive without letting it perturb the current genesis path.
