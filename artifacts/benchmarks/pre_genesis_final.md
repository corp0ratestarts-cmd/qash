# Pre-Genesis RC Final Benchmark Summary

**Milestone:** Outcome B — RC-only milestone (PR #228)
**Date:** 2026-06-02
**Commit:** `6021ef63db07bb055411ffa2317c0f6743d48acd` (branch `claude/loving-goodall-5uq8o`)
**Platform:** x86_64-unknown-linux-gnu (Linux 6.18.5)
**Toolchain:** rustc 1.95.0 (59807616e 2026-04-14)

This document archives the final performance evidence for the v1.0 RC milestone.
`GENESIS_CONSTANTS.toml` remains `genesis_status = "provisional"`,
`deployment_authoritative = false`. No `v1.0-reference` tag has been created.

---

## Control-loop budget gate

| Metric | Value | Budget | Result |
|--------|-------|--------|--------|
| Max epoch duration | 500 ms | — | |
| Max control-loop latency | 450 ms | 450 ms | **PASS** |
| 1024-validator epoch transition (idle) | ~312 µs | 450 ms | **≫ PASS** (1440× margin) |
| 1024-validator epoch transition (max divergence) | ~317 µs | 450 ms | **≫ PASS** (1419× margin) |
| 1024-validator TX-heavy advance (full batch, sort) | ~10.5 ms | 450 ms | **PASS** (43× margin) |

The worst-case 1024-validator scenario consumes < 0.1% of the epoch budget.

---

## epoch_transition (from `epoch_transition_baseline_9f6e995.md`, 2026-05-27)

| Benchmark | Validators | ns/iter | ± |
|-----------|-----------|---------|---|
| idle | 1 | 25,545 | 648 |
| max_divergence | 1 | 25,354 | 336 |
| idle | 16 | 29,888 | 1,321 |
| max_divergence | 16 | 29,635 | 251 |
| idle | 128 | 60,847 | 669 |
| max_divergence | 128 | 60,808 | 330 |
| idle | 512 | 169,437 | 7,381 |
| max_divergence | 512 | 168,230 | 1,650 |
| **idle** | **1024** | **311,881** | 3,076 |
| **max_divergence** | **1024** | **316,568** | 12,461 |

---

## Phase 2-R inclusion statement

Phase 2-R (Core Runtime Optimization, ADR-006) is included in this RC. The O(n log n)
sort replacement for TX admission was merged (commit `d729fcc`) and its benchmark
evidence is archived in `epoch_transition_sort_d729fcc.md`. The 1024-validator
TX-heavy worst case improved from ~12.5 ms to ~10.5 ms.

---

## serialization

| Benchmark | Validators | ns/iter | ± |
|-----------|-----------|---------|---|
| encode_full_state | 1 | 17 | 0 |
| decode_full_state | 1 | 113 | 1 |
| encode_full_state | 128 | 714 | 5 |
| decode_full_state | 128 | 1,041 | 17 |
| encode_full_state | 1024 | 7,697 | 33 |
| decode_full_state | 1024 | 8,194 | 90 |

---

## Source benchmark files

| File | Commit | Date | Notes |
|------|--------|------|-------|
| `epoch_transition_baseline_9f6e995.md` | `9f6e995` | 2026-05-27 | Baseline before Phase 2-R sort |
| `epoch_transition_sort_d729fcc.md` | `d729fcc` | 2026-05-29 | Phase 2-R TX-sort ADR-006 gate |
| `2026-05-19T-epoch-transition-x86_64.txt` | `60c0490` | 2026-05-19 | Early Wave 2 capture |
