# ADR-009: Domain B Indexing and ZK Prover Sizing

**Status:** Proposed
**Date:** 2026-05-22
**Source:** PR #93 LEANN and inference-performance review
**PDF authority:** PDF-SILENT

## Context

The latest PR #93 comments evaluate LEANN and LLM-serving performance methods
against QASH's sharding and proof pipeline.

LEANN is not a sharding or consensus mechanism. It is a vector-search storage
optimization and lacks deterministic state partitioning, validity proofs, and
consensus semantics. It can be considered only in Domain B for auditor-facing
receipt or compliance metadata indexing.

LLM-serving benchmark methods are also not directly applicable, but their
capacity-planning discipline is useful for ZK prover infrastructure.

## Decision

Schedule two Domain B-only work items:

- `ReceiptIndexer`: optional LEANN-backed indexing for encrypted receipts,
  compliance metadata, and off-chain auditor queries. The index must never
  publish private graph topology or affect Domain A ordering, roots, or replay.
- ZK prover sizing methodology for the Phase 6 sharding pipeline. Benchmark
  reports must use warm-up, progressive load testing, realistic transaction
  distributions, and a Pareto frontier for proof latency versus throughput.

The baseline sharding architecture remains static ZK shards plus EFB:

- static or genesis-profiled shard assignment before any dynamic sharding work
- asynchronous cross-shard receipt proofs, not atomic commit
- STARK proving and prover capacity planning in Domain B
- beacon aggregation and public-root commitment through the EFB surface

## Required Evidence

- LEANN or any alternative indexer must be behind PAL storage interfaces and
  absent from Domain A dependencies.
- Indexer tests must prove encrypted receipt blobs remain opaque and no PII is
  emitted to public storage.
- `docs/benchmarks/zk_prover_sizing.md` must record the chosen operating point,
  proof generation time, verification time, batch size, replica count, and
  hardware headroom.
- A sizing estimator must account for circuit size, trace buffer, per-proof
  overhead, and 20 percent headroom.
- Phase 6 cannot claim 10K+ TPS or sub-50ms finality until archived prover
  benchmark artifacts support the claim.
