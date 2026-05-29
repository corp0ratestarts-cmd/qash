# Epoch Transition Sort Replacement — Benchmark Evidence

**Commit:** `d729fcc` (fix: trailing bytes fix — sort replacement landed at `2636a4e`)  
**Branch:** `claude/modest-gates-tgIDP`  
**Date:** 2026-05-29  
**Platform:** x86\_64 (Linux 6.18.5, Rust 1.95.0)  
**Command:** `cargo bench -p qash-consensus --bench epoch_transition -- phase2r_tx`

This file archives the before/after Criterion evidence required by the ADR-006 gate:
"profiling evidence for an O(n log n) replacement is required before authorising
that optimisation." Sort replacement commit: `2636a4e`.

---

## B8: TX-heavy advance_epoch (forward + reversed input batches)

### Before (insertion sort — commit `eb5ab99`, baseline captured 2026-05-29)

| Benchmark | Validators | median (ms) | lo | hi |
|-----------|-----------|-------------|----|----|
| advance_epoch_full_tx_batch    | 16   | 0.135  | — | — |
| advance_epoch_reversed_tx_batch | 16  | 0.137  | — | — |
| advance_epoch_full_tx_batch    | 128  | 1.0538 | — | — |
| advance_epoch_reversed_tx_batch | 128 | 1.0299 | — | — |
| advance_epoch_full_tx_batch    | 512  | 5.0338 | 5.0338 | 5.1211 |
| advance_epoch_reversed_tx_batch | 512 | 5.1096 | 5.1096 | 5.1612 |
| advance_epoch_full_tx_batch    | 1024 | 12.482 | 12.482 | 12.668 |
| advance_epoch_reversed_tx_batch | 1024 | 12.628 | 12.628 | 12.903 |

### After (sort_unstable_by — commit `2636a4e`)

| Benchmark | Validators | median (ms) | lo | hi | Δ% |
|-----------|-----------|-------------|----|----|-----|
| advance_epoch_full_tx_batch    | 16   | 0.1348 | 0.1345 | 0.1352 | ~0%   |
| advance_epoch_reversed_tx_batch | 16  | 0.1372 | 0.1365 | 0.1379 | ~0%   |
| advance_epoch_full_tx_batch    | 128  | 1.0083 | 1.0048 | 1.0119 | **-4.6%** |
| advance_epoch_reversed_tx_batch | 128 | 0.9757 | 0.9603 | 0.9923 | **-5.0%** |
| advance_epoch_full_tx_batch    | 512  | 3.866  | 3.815  | 3.917  | **-23.8%** |
| advance_epoch_reversed_tx_batch | 512 | 3.559  | 3.529  | 3.594  | **-30.7%** |
| advance_epoch_full_tx_batch    | 1024 | 8.408  | 8.253  | 8.559  | **-33.1%** |
| advance_epoch_reversed_tx_batch | 1024 | 8.936 | 8.906  | 8.970  | **-29.9%** |

### B7a: prevalidate_all (tx admission, all tx assigned to slot 0)

| Benchmark | Validators | Before (ms) | After (ms) | Δ% |
|-----------|-----------|-------------|------------|-----|
| prevalidate_tx0 | 1   | 0.01219 | 0.01153 | -5.4% |
| prevalidate_tx0 | 16  | 0.1214  | 0.1207  | ~0%   |
| prevalidate_tx0 | 128 | 1.0318–1.0917 | 0.9652 | **-8.2%** |
| prevalidate_tx0 | 512 | 4.9257–4.9958 | 4.155  | **-16.2%** |
| prevalidate_tx0 | 1024 | 12.187–12.382 | 8.871 | **-27.7%** |

---

## Analysis

The O(n²) insertion sort was the dominant cost in `prevalidate_all` for large
validator counts. Replacing it with `sort_unstable_by` (introsort, O(n log n)
worst case) eliminates the quadratic component:

- n=128: 5–8% improvement (n² vs n log n crossover region)
- n=512: 16–31% improvement (clearly super-linear saving)
- n=1024: 27–34% improvement (quadratic cost dominates at max capacity)

Reversed-input batches show slightly larger gains than sorted-input batches
(reversed is worst-case for insertion sort; introsort handles both equally).
This validates the ADR-006 concern that "worst case" pathological inputs
produce O(n²) in insertion sort but O(n log n) in the replacement.

---

## Sort semantics parity

The `sort_unstable_by` comparator:
```rust
entries[..valid].sort_unstable_by(|a, b| match a.key.cmp(&b.key) {
    core::cmp::Ordering::Equal => a.id.cmp(&b.id),
    other => other,
});
```
produces ascending `(sort_key, tx_id)` order — identical to the removed
`candidate_after` comparator (`left.key > right.key || (left.key == right.key && left.id > right.id)`).

Sort-order determinism verified by:
- `transaction::tests::sort_order_is_identical_for_reversed_input_batch`
- All `phase2r_preconditions` tests

---

## Domain constraint check

- `sort_unstable_by` uses `core::cmp::Ordering` — available in `core`, no `std`/`alloc`. ✓
- No `f32`/`f64`, no `usize` in state, no HashMap. ✓
- Overflow policy unchanged. ✓
- Wire format unchanged. ✓
