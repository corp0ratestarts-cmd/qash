# Epoch Transition Benchmark Baseline

**Commit:** `9f6e995286c58ae6306811399947e09520d7daa4`  
**Branch:** `claude/track-7-streaming-state-root-parity`  
**Date:** 2026-05-27  
**Platform:** x86\_64 (Linux 6.18.5)  
**Command:** `cargo bench -p qash-consensus 2>&1`

This baseline was captured at the Track 7 streaming state-root parity commit (6 parity tests added).
No protocol changes were made; this is a pure benchmark capture for Track 7 ADR evidence.

---

## epoch_transition

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
| idle | 1024 | 311,881 | 3,076 |
| max_divergence | 1024 | 316,568 | 12,461 |

**1024-validator epoch transition: ~312 µs (idle), ~317 µs (max divergence)**

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

## replay

| Benchmark | Config | ns/iter | ± |
|-----------|--------|---------|---|
| idle_4v | 10 epochs | 243,326 | 17,335 |
| idle_4v | 100 epochs | 2,340,792 | 20,027 |
| idle_4v | 500 epochs | 11,677,305 | 119,426 |
| idle_1024v | 10 epochs | 3,145,124 | 60,713 |

---

## hash

| Benchmark | Bytes | ns/iter | ± |
|-----------|-------|---------|---|
| h_domain_sha3 | 32 | 464 | 6 |
| h_domain_sha3 | 512 | 1,680 | 18 |
| h_domain_sha3 | 4,096 | 12,413 | 100 |

---

## phase2r_tx_admission (prevalidate_tx0)

| Benchmark | TXs | ns/iter | ± |
|-----------|-----|---------|---|
| prevalidate_tx0 | 1 | 13,248 | 115 |
| prevalidate_tx0 | 16 | 136,532 | 4,821 |
| prevalidate_tx0 | 128 | 1,143,630 | 18,390 |
| prevalidate_tx0 | 512 | 5,584,318 | 95,949 |
| prevalidate_tx0 | 1024 | 13,833,602 | 132,421 |

---

## phase2r_validator_lookup (prevalidate_single_tx0)

| Benchmark | Position | ns/iter | ± |
|-----------|----------|---------|---|
| prevalidate_single_tx0 | first | 14,064 | 228 |
| prevalidate_single_tx0 | middle | 15,429 | 301 |

---

## ADR Evidence Notes

- All benchmarks are single-pass (no `--no-default-features` exclusions needed; bench suite is feature-independent)
- Wire format, hash preimage, and Domain A rule set are **unchanged** by Track 7 parity tests
- No proof obligations were modified
- These numbers serve as the pre-ProjectedView baseline for Track 7 item 5 (runtime-only ProjectedView)
- Cross-ISA parity (aarch64, riscv64gc) is required before any throughput claims; these x86_64 numbers are reference only
