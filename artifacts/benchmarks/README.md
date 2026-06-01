# Benchmark Artifacts

This directory holds archived Criterion benchmark results for the QASH
v1.0 genesis-candidate performance evidence (Wave 5 PR #235).

## Coverage

The benchmark suites under `crates/consensus/benches/` and `crates/pal/benches/`
cover:

| Benchmark | Suite | Scenario |
|-----------|-------|---------|
| `epoch_transition/idle` | consensus | 1/16/128/512/1024 validators, zero metrics |
| `epoch_transition/max_divergence` | consensus | 1/16/128/512/1024 validators, max divergence |
| `serialization/encode_full_state` | consensus | 1/128/1024 validators |
| `serialization/decode_full_state` | consensus | 1/128/1024 validators |
| `replay/idle_4v` | consensus | 10/100/500 epochs, 4 validators |
| `replay/idle_1024v_10epochs` | consensus | 10 epochs, 1024 validators |
| `hash/h_domain_sha3` | consensus | 32/512/4096 byte inputs |
| `phase2r_tx_admission/prevalidate_tx0` | consensus | 1–1024 tx batch sizes |
| `phase2r_validator_lookup` | consensus | first/middle/last validator in 1024-validator set |
| `phase2r_state_root_commitment` | consensus | 1/128/1024 validators |
| `phase2r_epoch_advancement_baseline` | consensus | 128/1024 validators |
| `phase2r_tx_heavy_advance` | consensus | forward+reverse 16–1024 tx batches |
| `max_validators_state_copy` | consensus | 128/512/1024 validator struct copy |
| `dual_hash_32` | pal | 64/1024/65536 byte inputs |
| `allof_hash_pair_32` | pal | 64/1024/65536 byte inputs |
| `allof_manifest_root/1000_chunk_hashes` | pal | 1000-chunk all-of manifest |

## How to capture

```sh
# Consensus benchmarks
cargo bench -p qash-consensus -- --output-format bencher 2>&1 \
  | tee artifacts/benchmarks/$(date -u +%Y%m%dT%H%M%SZ)-consensus.txt

# PAL benchmarks
cargo bench -p qash-pal -- --output-format bencher 2>&1 \
  | tee artifacts/benchmarks/$(date -u +%Y%m%dT%H%M%SZ)-pal.txt
```

## Performance gate (Phase 2-R)

Any optimisation in PR #236 must:
1. Produce byte-identical state roots (KAT vectors unchanged)
2. Show a measurable improvement on `phase2r_tx_heavy_advance` or
   `epoch_transition/max_divergence` at vc=1024
3. Not regress `max_validators_state_copy` for vc=1024
4. Keep cross-ISA determinism unchanged (genesis hash invariant)
