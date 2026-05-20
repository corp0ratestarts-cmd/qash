use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use qash_consensus::{
    advance_epoch, compute_sort_key, DomainTag, EpochInput, EpochState, FixedPoint, HaltReason,
    ValidatorMetrics, MAX_VALIDATORS,
};
use rand::{rngs::StdRng, RngCore, SeedableRng};
use std::time::{Duration, Instant};

const BENCH_EPOCHS: u64 = 100;
const BENCH_ENVELOPES_PER_EPOCH: usize = 50;
const CASCADE_HEALTH_THRESHOLD: u32 = 8;
const COMPATIBILITY_WINDOW: u64 = 100;
const PAYLOAD_BYTES: usize = 256;

fn create_initial_state() -> EpochState {
    EpochState {
        epoch: 0,
        halt_reason: HaltReason::None,
        entropy_seed: [0u8; 32],
        validators: [ValidatorMetrics::ZERO; MAX_VALIDATORS],
        validator_count: 4,
        convergence_window: qash_consensus::lyapunov::ConvergenceWindow::new(),
        nonces: [0u64; MAX_VALIDATORS],
        validator_ids: [[0u8; 48]; MAX_VALIDATORS],
        cascade_health: 0,
        state_root: [0u8; 32],
    }
}

fn idle_input(state: &EpochState) -> EpochInput {
    EpochInput { updates: [None; MAX_VALIDATORS], update_count: state.validator_count }
}

fn generate_sort_key(rng: &mut StdRng, epoch_seed: &[u8; 32]) -> [u8; 32] {
    let mut payload = [0u8; PAYLOAD_BYTES];
    rng.fill_bytes(&mut payload);
    let envelope_hash = qash_consensus::h_domain(DomainTag::Envelope, &payload);
    compute_sort_key(epoch_seed, 0, &envelope_hash)
}

fn bench_envelope_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("envelope_processing");
    group.throughput(Throughput::Elements(1));
    let mut rng = StdRng::seed_from_u64(42);

    group.bench_function(BenchmarkId::new("admit_single_envelope", "v1.1"), |b| {
        b.iter(|| {
            let mut state = create_initial_state();
            let input = idle_input(&state);
            let tx = [0u8; 8];
            let start = Instant::now();
            let _ = advance_epoch(black_box(&mut state), black_box(&input), black_box(&[&tx[..]]));
            let elapsed = start.elapsed();
            assert!(elapsed < Duration::from_millis(2));
            black_box(elapsed)
        });
    });

    group.bench_function(BenchmarkId::new("admit_batch_50", "v1.1"), |b| {
        b.iter(|| {
            let mut state = create_initial_state();
            let input = idle_input(&state);
            let txs: Vec<[u8; 8]> = (0..BENCH_ENVELOPES_PER_EPOCH).map(|_| rng.next_u64().to_le_bytes()).collect();
            let tx_refs: Vec<&[u8]> = txs.iter().map(|t| &t[..]).collect();
            let start = Instant::now();
            let _ = advance_epoch(&mut state, &input, &tx_refs);
            let elapsed = start.elapsed();
            let per = elapsed / BENCH_ENVELOPES_PER_EPOCH as u32;
            assert!(per < Duration::from_millis(2));
            black_box(elapsed)
        });
    });
    group.finish();
}

fn bench_epoch_finality(c: &mut Criterion) {
    let mut group = c.benchmark_group("epoch_finality");
    group.bench_function(BenchmarkId::new("finality_decision_latency", "v1.1"), |b| {
        b.iter(|| {
            let mut state = create_initial_state();
            state.cascade_health = CASCADE_HEALTH_THRESHOLD - 1;
            state.epoch = COMPATIBILITY_WINDOW;
            let input = idle_input(&state);
            let start = Instant::now();
            let _ = advance_epoch(&mut state, &input, &[]);
            let elapsed = start.elapsed();
            assert!(elapsed < Duration::from_millis(50));
            black_box(elapsed)
        });
    });
    group.finish();
}

fn bench_cross_isa_determinism(c: &mut Criterion) {
    let mut group = c.benchmark_group("cross_isa_determinism");
    let mut rng = StdRng::seed_from_u64(42);
    group.bench_function("state_root_consistency", |b| {
        b.iter(|| {
            let mut state = create_initial_state();
            for _ in 0..10 {
                let _ = generate_sort_key(&mut rng, &state.entropy_seed);
                let input = idle_input(&state);
                let _ = advance_epoch(&mut state, &input, &[]);
            }
            black_box(state.state_root)
        });
    });
    group.finish();
}

fn bench_tail_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("tail_latency");
    group.bench_function("epoch_step_p99_latency", |b| {
        let mut latencies = Vec::with_capacity(1000);
        b.iter(|| {
            latencies.clear();
            for _ in 0..1000 {
                let mut state = create_initial_state();
                let input = idle_input(&state);
                let start = Instant::now();
                let _ = advance_epoch(&mut state, &input, &[]);
                latencies.push(start.elapsed());
            }
            latencies.sort();
            let p99 = latencies[(latencies.len() as f64 * 0.99) as usize];
            assert!(p99 < Duration::from_millis(2));
            black_box(p99)
        });
    });
    group.finish();
}

criterion_group!(benches, bench_envelope_processing, bench_epoch_finality, bench_cross_isa_determinism, bench_tail_latency);
criterion_main!(benches);
