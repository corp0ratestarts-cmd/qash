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
- `phase2r_state_root_commitment_matches_buffered_preimage` and
  `phase2r_streaming_state_root_matches_buffered_for_varied_states` in
  `crates/consensus/tests/phase2r_preconditions.rs` — exact preimage parity
  between the streaming and buffered `encode_full_state_into` paths across
  varied validator counts and epoch depths.

## Sort-order determinism (Track 7 addition, 2026-05-26)

`transaction::tests::sort_order_is_identical_for_reversed_input_batch` verifies
that `prevalidate_all` produces identical `applied_count` and `next_nonces`
regardless of whether the raw-tx slice is `[tx_a, tx_b]` or `[tx_b, tx_a]`.
This is the required reversed-input determinism evidence cited in ADR-006
§"Required Evidence". The insertion-sort over `(sort_key, tx_id)` is O(n²)
in the worst case for n ≤ 1024 — profiling evidence for an O(n log n)
replacement is required before authorising that optimisation.

## Consensus transition semantics

Unchanged. `advance_epoch` and `advance_epoch_sharded` in `qash-consensus` have identical signatures and behaviour. No `GENESIS_CONSTANTS.toml` entries were modified. The `advance_epoch` function is called with the same `EpochInput` records in the same order as before.

## Domain boundary assertion

All changes in Phase 2-R are confined to Domain B:

- `crates/pal/src/mvp_vault.rs` — WAL read loop and vault replay logic (Domain B).
- `src/demo.rs` — CLI replay adapter now routes through `qash_pal::mvp_demo_profile::replay_public_export_bytes` (Domain B).

No files under `crates/consensus/` were modified. No Domain A types, state fields, or arithmetic expressions were changed. Cross-domain contamination check: no Domain B value flows into a Domain A computation as a result of these changes.

## Remaining 2-R work

| Item | Status | Gate |
|------|--------|------|
| O(n log n) sort replacement | **Landed** (branch `claude/modest-gates-tgIDP`, commit `2636a4e`) | Evidence archived at `artifacts/benchmarks/epoch_transition_sort_d729fcc.md` |
| `ProjectedView` struct | **Landed** (PR #183) | Parity verified by 3 tests |
| Validator directory | Deferred | Only if profiling shows lookup cost dominates |
| Cross-ISA parity for PR #167 byte-read path | Open | Requires aarch64/riscv64gc CI run |

---

# ADR-006 Phase 2-R Evidence Note (Domain A — Track 7, 2026-05-27)

**Scope:** Domain A (`crates/consensus/src/transition.rs`)  
**Branch:** `claude/track-7-streaming-state-root-parity`

## Changes

### Streaming state-root computation (Domain A, `transition.rs`)

The state-root commitment path was refactored to stream field groups directly into
SHA3-256 via incremental `update()` calls, replacing a 82 KB intermediate stack buffer
(`FULL_STATE_MAX_BYTES = 82,132` bytes). The domain tag, field ordering, and byte layout
are identical. Only the allocation pattern changed.

Six parity tests (`streaming_state_root_parity_*`) verify byte-identical output across
all state shapes (genesis, non-zero prior root, non-zero validator metrics, receipt_root
only, both sharding roots, maximum validators). All pass.

### ProjectedView (Domain A, `transition.rs`)

A private, runtime-only `ProjectedView<'a>` struct was introduced to eliminate the ~80 KB
`EpochState` copy previously made in `run_pipeline` before computing the state root.

The struct holds:
- References to the unchanged large arrays (`validator_ids`) borrowed from `state`
- References to freshly-computed arrays already on the stack (`validators`, `nonces`)
- Owned scalars for the eight fields updated during the transition

`ProjectedView::compute_root` replicates `stream_state_for_commitment` field-by-field and
is verified by three parity tests (`projected_view_compute_root_matches_full_state_*`). All
pass. The commit-point direct assignments to `*state` are unchanged.

### Benchmark baseline

Archived at `artifacts/benchmarks/epoch_transition_baseline_9f6e995.md` (commit `9f6e995`).
Key figure: 1024-validator `advance_epoch` ~312 µs on x86\_64. Cross-ISA verification required
before any external throughput claims.

## Protocol boundary assertion

- Wire format: **unchanged**
- Hash preimage / domain tags: **unchanged**
- `advance_epoch` / `advance_epoch_sharded` signatures: **unchanged**
- `GENESIS_CONSTANTS.toml`: **unchanged**
- Coq proof files: **unchanged**
- `ProjectedView` is not exported from the crate and has no protocol-facing presence
- Cross-domain contamination: no Domain B value introduced or changed

---

# ADR-006 Phase 2-R Evidence Note (Domain A — Sort Replacement, 2026-05-29)

**Scope:** Domain A (`crates/consensus/src/transaction.rs`)  
**Branch:** `claude/modest-gates-tgIDP`, commit `2636a4e`

## Changes

### O(n log n) sort replacement

The O(n²) insertion sort over `CandidateTx` entries in `prevalidate_all` was replaced
with `entries[..valid].sort_unstable_by(|a, b| ...)` using `core::cmp::Ordering` — no
`std`, no alloc, Domain A safe.

The `candidate_after` comparator function was removed as dead code. The replacement
comparator is inline and produces identical `(sort_key, tx_id)` ascending order.

## Benchmark evidence

Full before/after numbers archived at `artifacts/benchmarks/epoch_transition_sort_d729fcc.md`.

Key improvements (n=1024, worst-case reversed input):
- `advance_epoch_reversed_tx_batch/1024`: 12.6 ms → 8.9 ms (**-29.9%**)
- `advance_epoch_full_tx_batch/1024`: 12.5 ms → 8.4 ms (**-33.1%**)
- `prevalidate_tx0/1024`: 12.2 ms → 8.9 ms (**-27.7%**)

Insertion sort is O(n²) worst-case for reversed input; `sort_unstable_by` (introsort)
is O(n log n) in all cases. The improvement scales super-linearly with n, confirming
the algorithmic root cause.

## Parity evidence

- `transaction::tests::sort_order_is_identical_for_reversed_input_batch` — same
  `applied_count` and `next_nonces` regardless of input order. Passes.
- All `phase2r_preconditions` tests pass.

## Domain boundary assertion

- Wire format: **unchanged**
- `GENESIS_CONSTANTS.toml`: **unchanged**
- Coq proof files: **unchanged**
- Cross-domain contamination: none
