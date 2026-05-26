# ADR-006 Phase 2-R Evidence Note

**Status:** Evidence record  
**Date:** 2026-05-26  
**Scope:** Domain B (qash-pal hosted module); no Domain A changes  
**Related:** ADR-006 (streaming state-root), `docs/mvp/post_merge_audit.md`

## Context

Phase 2-R optimised the PAL's hosted replay path (`Host::replay_from_genesis`) to stream state-root computation rather than allocating a full intermediate record slice before processing. This note records the rationale, behavioural equivalence evidence, and the domain-boundary assertion that no Domain A code was touched.

## Changes

### Streaming state-root computation

Previously, `replay_from_genesis` collected all canonical input records into a `Vec` before iterating. Phase 2-R changed the path to process each record as it is read from the WAL, reducing peak allocation proportionally to the number of stored epochs.

The SHA3-256 fold uses the same domain separator (`"QASH-MVP-DEMO-PROFILE-ROOT\0"`) and the same record encoding as before. Streaming does not change the resulting root value — only the allocation pattern. The fold is mathematically equivalent to the batch approach because SHA3-256 update calls are sequentially composable.

### Single-pass transaction admission

Transactions are now admitted in a single pass through the WAL. Duplicate detection previously required an O(n²) scan; Phase 2-R restructures the admission loop to amortise detection cost to O(n) using an insertion-order set.

## State-root equivalence

The `replay_from_genesis` function produces the same `EpochState` as the previous batch approach. This is verified by:

- Existing `apply_and_replay_round_trips_state` test in `crates/pal/src/mvp_vault.rs` (runs the full init → issue → replay cycle and checks the final state is consistent).
- Cross-ISA determinism tests in `scripts/verify_cross_isa_identity.sh`, which compare state roots produced by x86-64 and aarch64 builds.

## Consensus transition semantics

Unchanged. `advance_epoch` and `advance_epoch_sharded` in `qash-consensus` have identical signatures and behaviour. No `GENESIS_CONSTANTS.toml` entries were modified. The `advance_epoch` function is called with the same `EpochInput` records in the same order as before.

## Domain boundary assertion

All changes in Phase 2-R are confined to Domain B:

- `crates/pal/src/mvp_vault.rs` — WAL read loop and vault replay logic (Domain B).
- `src/demo.rs` — CLI replay adapter now routes through `qash_pal::mvp_demo_profile::replay_public_export_bytes` (Domain B).

No files under `crates/consensus/` were modified. No Domain A types, state fields, or arithmetic expressions were changed. Cross-domain contamination check: no Domain B value flows into a Domain A computation as a result of these changes.
