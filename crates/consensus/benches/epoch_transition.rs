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
    fixed_point::FixedPoint,
    lyapunov::{ConvergenceWindow, ValidatorMetrics},
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
    }
}

fn idle_input(vc: u32) -> EpochInput {
    EpochInput {
        updates: [None; MAX_VALIDATORS],
        protocol_version: qash_consensus::envelope::PROTOCOL_VERSION_V1_1,
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
    use qash_consensus::hash::{h_domain, DomainTag};

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

criterion_group!(
    benches,
    bench_epoch_transition,
    bench_serialization,
    bench_replay,
    bench_hash
);
criterion_main!(benches);
