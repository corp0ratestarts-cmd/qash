# ADR-009: Domain B Indexing and ZK Prover Sizing

**Status:** Proposed  
**Date:** 2026-05-24  
**Source:** PR #93 follow-through reconciliation

## Context

PR #93 raised LEANN/vector indexing and ZK prover capacity concerns. Both are
valid operational concerns, but neither belongs in Domain A consensus semantics.

## Decision

Domain B may use LEANN, vector indexes, or equivalent retrieval structures only
for encrypted receipt metadata, compliance search, auditor workflows, and local
operator tooling. Such indexes must not drive sharding, consensus order,
validator selection, finality, halt logic, or public transcript semantics.

No throughput, TPS, or finality-latency claim is accepted until ZK prover sizing
benchmarks are archived for the exact implementation commit.

## Required constraints

- No Domain A dependency on LEANN, vector DBs, RAG stores, or retrieval indexes.
- No PII, graph topology, raw receipts, or peer metadata emitted through indexes.
- Index entries must reference commitments or encrypted metadata only.
- Index state is rebuildable Domain B state, not consensus state.
- ZK prover sizing must include circuit size, trace buffers, per-proof overhead,
  replica count, GPU/accelerator count, warm-up behavior, and at least 20 percent
  memory headroom.

## Required evidence

- `docs/benchmarks/zk_prover_sizing.md` before any 10K+ TPS or sub-50ms claim.
- No-Domain-A-dependency static checks.
- Privacy review for index schemas.
- Benchmark artifacts under `artifacts/benchmarks/`.

## Consequences

Indexing and prover sizing become production-engineering tracks without changing
QASH's deterministic consensus kernel.
