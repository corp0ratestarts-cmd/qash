// Criterion benchmarks for qash-consensus Domain A hot paths.
//
// Scenarios (PROJECT_STATUS.md Phase 2 item 6):
//   B1: Worst-case epoch transition — 1024 validators, max divergence.
//   B2: Idle epoch transition — 1024 validators, zero metrics.
//   B3: Single-validator epoch transition (baseline).
//   B4: Full-state encoding — 1024 validators.
//   B5: Full-state decoding — 1024 validators.
//   B6: 100-epoch replay — 4 validators, accumulating entropy.
//
// Run: cargo bench -p qash-consensus (requires `default-features` for std timer)
//
// To capture evidence under artifacts/benchmarks/:
//   cargo bench -p qash-consensus -- --output-format bencher 2>&1 \
//     | tee artifacts/benchmarks/$(date -u +%Y%m%dT%H%M%SZ)-$(rustc --version | awk '{print $2}').txt

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use qash_consensus::{
    envelope::PROTOCOL_VERSION_V1_1,
    fixed_point::FixedPoint,
    hash::{h_domain, DomainTag},
    lyapunov::{ConvergenceWindow, ValidatorMetrics},
    transaction::{prevalidate_all, TX0_WIRE_BYTES, TX_HEADER_BYTES, TX_TYPE_NOOP, TX_VERSION},
    transition::{
        advance_epoch, decode_full_state, encode_full_state_into, EpochInput, EpochState,
        HaltReason, ValidatorUpdate, FULL_STATE_MAX_BYTES, MAX_VALIDATORS,
    },
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_state(vc: u32) -> EpochState {
    let mut validator_ids = [[0u8; 48]; MAX_VALIDATORS];
    for i in 0..vc as usize {
        validator_ids[i][0..8].copy_from_slice(&(i as u64 + 1).to_le_bytes());
    }
    EpochState {
        epoch: 0,
        halt_reason: HaltReason::None,
        entropy_seed: [0xABu8; 32],
        validators: [ValidatorMetrics::ZERO; MAX_VALIDATORS],
        validator_count: vc,
        convergence_window: ConvergenceWindow::new(),
        nonces: [0u64; MAX_VALIDATORS],
        validator_ids,
        cascade_health: 0,
        state_root: [0u8; 32],
        receipt_root: [0u8; 32],
        efb_root: [0u8; 32],
        causal_fingerprint: [0u8; 32],
    }
}

fn idle_input(vc: u32) -> EpochInput {
    EpochInput {
        updates: [None; MAX_VALIDATORS],
        protocol_version: PROTOCOL_VERSION_V1_1,
        update_count: vc,
    }
}

fn max_divergence_input(vc: u32) -> EpochInput {
    let scale = 1_000_000i128;
    let mut input = EpochInput {
        updates: [None; MAX_VALIDATORS],
        protocol_version: qash_consensus::envelope::PROTOCOL_VERSION_V1_1,
        update_count: vc,
    };
    for i in 0..vc as usize {
        input.updates[i] = Some(ValidatorUpdate {
            divergence_new: FixedPoint::from_raw(scale),
            conflict_new: FixedPoint::from_raw(scale / 2),
            slash_accum_new: FixedPoint::from_raw(scale / 10),
        });
    }
    input
}

fn make_tx0_raw(author_id: [u8; 48], tx_sequence: u64, signature_byte: u8) -> [u8; TX0_WIRE_BYTES] {
    let mut raw = [0u8; TX0_WIRE_BYTES];
    raw[0..2].copy_from_slice(&TX_VERSION.to_le_bytes());
    raw[2..4].copy_from_slice(&TX_TYPE_NOOP.to_le_bytes());
    raw[4..12].copy_from_slice(&tx_sequence.to_le_bytes());
    raw[12..60].copy_from_slice(&author_id);
    raw[60..64].copy_from_slice(&0u32.to_le_bytes());
    raw[TX_HEADER_BYTES] = signature_byte;
    raw
}

fn tx_batch(state: &EpochState, count: usize) -> Vec<[u8; TX0_WIRE_BYTES]> {
    state
        .validator_ids
        .iter()
        .take(count)
        .enumerate()
        .map(|(i, author)| make_tx0_raw(*author, 0, i as u8))
        .collect()
}

// ---------------------------------------------------------------------------
// B1 + B2: Epoch transition throughput
// ---------------------------------------------------------------------------

fn bench_epoch_transition(c: &mut Criterion) {
    let mut group = c.benchmark_group("epoch_transition");

    for &vc in &[1u32, 16, 128, 512, 1024] {
        // B2-variant: idle (zero metrics, no Lyapunov pressure)
        group.bench_with_input(BenchmarkId::new("idle", vc), &vc, |b, &vc| {
            let state_init = make_state(vc);
            let input = idle_input(vc);
            b.iter(|| {
                let mut state = state_init;
                let _ = advance_epoch(black_box(&mut state), black_box(&input), black_box(&[]));
                black_box(state.state_root)
            });
        });

        // B1-variant: max divergence (all validators at D=1, C=0.5, S=0.1)
        // The Lyapunov window is not full on epoch 0 so no halt triggers here;
        // this measures the worst-case arithmetic path.
        group.bench_with_input(BenchmarkId::new("max_divergence", vc), &vc, |b, &vc| {
            let state_init = make_state(vc);
            let input = max_divergence_input(vc);
            b.iter(|| {
                let mut state = state_init;
                let _ = advance_epoch(black_box(&mut state), black_box(&input), black_box(&[]));
                black_box(state.state_root)
            });
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// B4 + B5: Full-state serialization throughput
// ---------------------------------------------------------------------------

fn bench_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization");

    for &vc in &[1u32, 128, 1024] {
        // Encoding
        group.bench_with_input(BenchmarkId::new("encode_full_state", vc), &vc, |b, &vc| {
            let state = make_state(vc);
            let mut buf = [0u8; FULL_STATE_MAX_BYTES];
            b.iter(|| {
                let n = encode_full_state_into(black_box(&state), black_box(&mut buf));
                black_box(n)
            });
        });

        // Decoding
        group.bench_with_input(BenchmarkId::new("decode_full_state", vc), &vc, |b, &vc| {
            let state = make_state(vc);
            let mut buf = [0u8; FULL_STATE_MAX_BYTES];
            let n = encode_full_state_into(&state, &mut buf);
            let encoded = &buf[..n];
            b.iter(|| {
                let result = decode_full_state(black_box(encoded));
                black_box(result.is_ok())
            });
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// B6: N-epoch replay latency
// ---------------------------------------------------------------------------

fn bench_replay(c: &mut Criterion) {
    let mut group = c.benchmark_group("replay");

    for &epochs in &[10u64, 100, 500] {
        group.bench_with_input(
            BenchmarkId::new("idle_4v", epochs),
            &epochs,
            |b, &epochs| {
                b.iter(|| {
                    let mut state = make_state(4);
                    let input = idle_input(4);
                    for _ in 0..epochs {
                        let _ = advance_epoch(&mut state, &input, &[]);
                    }
                    black_box(state.state_root)
                });
            },
        );
    }

    // Worst-case: 1024 validators, 10 idle epochs (as a replay cost proxy)
    group.bench_function("idle_1024v_10epochs", |b| {
        b.iter(|| {
            let mut state = make_state(1024);
            let input = idle_input(1024);
            for _ in 0..10 {
                let _ = advance_epoch(&mut state, &input, &[]);
            }
            black_box(state.state_root)
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// B7: Hash cascade throughput (Domain A state root commitment)
// ---------------------------------------------------------------------------

fn bench_hash(c: &mut Criterion) {
    let mut group = c.benchmark_group("hash");

    for &len in &[32usize, 512, 4096] {
        let data: Vec<u8> = (0..len).map(|i| i as u8).collect();
        group.bench_with_input(BenchmarkId::new("h_domain_sha3", len), &data, |b, data| {
            b.iter(|| {
                let result = h_domain(black_box(DomainTag::StateRoot), black_box(data.as_slice()));
                black_box(result)
            });
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Phase 2-R precondition benches: tx admission, validator lookup, sorting proxy,
// and state-root commitment.
// ---------------------------------------------------------------------------

fn bench_phase2r_tx_admission(c: &mut Criterion) {
    let mut group = c.benchmark_group("phase2r_tx_admission");

    for &count in &[1usize, 16, 128, 512, 1024] {
        group.bench_with_input(
            BenchmarkId::new("prevalidate_tx0", count),
            &count,
            |b, &count| {
                let state = make_state(count as u32);
                let txs = tx_batch(&state, count);
                let refs: Vec<&[u8]> = txs.iter().rev().map(|tx| tx.as_slice()).collect();
                b.iter(|| {
                    let plan = prevalidate_all(
                        black_box(&state),
                        black_box(refs.as_slice()),
                        black_box(count as u32),
                    );
                    black_box(plan.is_ok())
                });
            },
        );
    }

    group.finish();
}

fn bench_phase2r_validator_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("phase2r_validator_lookup");

    for &(label, slot) in &[("first", 0usize), ("middle", 511), ("last", 1023)] {
        group.bench_with_input(
            BenchmarkId::new("prevalidate_single_tx0", label),
            &slot,
            |b, &slot| {
                let state = make_state(1024);
                let tx = make_tx0_raw(state.validator_ids[slot], 0, slot as u8);
                let refs: [&[u8]; 1] = [tx.as_slice()];
                b.iter(|| {
                    let plan = prevalidate_all(black_box(&state), black_box(&refs), black_box(1));
                    black_box(plan.is_ok())
                });
            },
        );
    }

    group.finish();
}

fn bench_phase2r_state_root_commitment(c: &mut Criterion) {
    let mut group = c.benchmark_group("phase2r_state_root_commitment");

    for &vc in &[1u32, 128, 1024] {
        group.bench_with_input(
            BenchmarkId::new("buffered_commitment", vc),
            &vc,
            |b, &vc| {
                let state = make_state(vc);
                let mut buf = [0u8; FULL_STATE_MAX_BYTES];
                b.iter(|| {
                    let n = encode_full_state_into(black_box(&state), black_box(&mut buf));
                    let root = h_domain(DomainTag::StateRoot, black_box(&buf[..n]));
                    black_box(root)
                });
            },
        );
    }

    group.finish();
}

fn bench_phase2r_epoch_advancement_baseline(c: &mut Criterion) {
    let mut group = c.benchmark_group("phase2r_epoch_advancement_baseline");

    for &vc in &[128u32, 1024] {
        group.bench_with_input(BenchmarkId::new("advance_epoch_idle", vc), &vc, |b, &vc| {
            let state_init = make_state(vc);
            let input = idle_input(vc);
            b.iter(|| {
                let mut state = state_init;
                let result =
                    advance_epoch(black_box(&mut state), black_box(&input), black_box(&[]));
                black_box(result.is_ok())
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_epoch_transition,
    bench_serialization,
    bench_replay,
    bench_hash,
    bench_phase2r_tx_admission,
    bench_phase2r_validator_lookup,
    bench_phase2r_state_root_commitment,
    bench_phase2r_epoch_advancement_baseline
);
criterion_main!(benches);
