//! Domain A TPS smoke model.
//!
//! This executable measures the CPU-only Domain A hot path with release builds.
//! It intentionally does not use Criterion so it can print direct TPS-style
//! numbers for quick bottleneck triage and shard-capacity estimation.
//!
//! Run:
//!   cargo run -p qash-consensus --release --example domain_a_tps_smoke -- \
//!     --iters 200 --warmup 20 --shards 1,4,16,64
//!
//! Notes:
//! - Domain A remains CPU-only by design. Hardware acceleration belongs in
//!   Domain B and must not alter state-root semantics.
//! - Shard TPS below is a linear independent-shard capacity model, not a claim
//!   of production network throughput or global finality.

use qash_consensus::{
    envelope::PROTOCOL_VERSION_V1_1,
    fixed_point::FixedPoint,
    lyapunov::{ConvergenceWindow, ValidatorMetrics},
    transaction::{TX0_WIRE_BYTES, TX_HEADER_BYTES, TX_TYPE_NOOP, TX_VERSION},
    transition::{
        advance_epoch, EpochInput, EpochState, HaltReason, ValidatorUpdate, MAX_VALIDATORS,
    },
};
use std::{env, time::Instant};

#[derive(Debug, Clone)]
struct Config {
    iters: usize,
    warmup: usize,
    shards: Vec<usize>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            iters: 200,
            warmup: 20,
            shards: vec![1, 4, 16, 64],
        }
    }
}

fn parse_config() -> Result<Config, String> {
    let mut cfg = Config::default();
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--iters" => {
                let raw = args.next().ok_or("--iters requires a value")?;
                cfg.iters = raw
                    .parse::<usize>()
                    .map_err(|_| "--iters must be a positive integer")?;
            }
            "--warmup" => {
                let raw = args.next().ok_or("--warmup requires a value")?;
                cfg.warmup = raw
                    .parse::<usize>()
                    .map_err(|_| "--warmup must be a non-negative integer")?;
            }
            "--shards" => {
                let raw = args.next().ok_or("--shards requires a comma list")?;
                let mut shards = Vec::new();
                for item in raw.split(',') {
                    let n = item
                        .trim()
                        .parse::<usize>()
                        .map_err(|_| "--shards entries must be positive integers")?;
                    if n == 0 {
                        return Err("--shards entries must be positive".into());
                    }
                    shards.push(n);
                }
                cfg.shards = shards;
            }
            "--help" | "-h" => {
                println!("usage: domain_a_tps_smoke [--iters N] [--warmup N] [--shards 1,4,16,64]");
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }

    if cfg.iters == 0 {
        return Err("--iters must be positive".into());
    }
    Ok(cfg)
}

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
        protocol_version: PROTOCOL_VERSION_V1_1,
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

fn tx_batch(state: &EpochState, count: usize, reversed: bool) -> Vec<[u8; TX0_WIRE_BYTES]> {
    let mut out: Vec<[u8; TX0_WIRE_BYTES]> = state
        .validator_ids
        .iter()
        .take(count)
        .enumerate()
        .map(|(i, author)| make_tx0_raw(*author, 0, i as u8))
        .collect();
    if reversed {
        out.reverse();
    }
    out
}

#[derive(Clone, Copy)]
enum Scenario {
    Idle,
    MaxDivergence,
    TxBatchOrdered,
    TxBatchReversed,
}

impl Scenario {
    fn name(self) -> &'static str {
        match self {
            Scenario::Idle => "idle",
            Scenario::MaxDivergence => "max_divergence",
            Scenario::TxBatchOrdered => "tx_batch_ordered",
            Scenario::TxBatchReversed => "tx_batch_reversed",
        }
    }

    fn tx_per_epoch(self, vc: u32) -> usize {
        match self {
            Scenario::TxBatchOrdered | Scenario::TxBatchReversed => vc as usize,
            Scenario::Idle | Scenario::MaxDivergence => 0,
        }
    }
}

fn run_scenario(vc: u32, scenario: Scenario, cfg: &Config) -> Result<(f64, f64, [u8; 32]), String> {
    let state_init = make_state(vc);
    let input = match scenario {
        Scenario::Idle | Scenario::TxBatchOrdered | Scenario::TxBatchReversed => idle_input(vc),
        Scenario::MaxDivergence => max_divergence_input(vc),
    };
    let txs = match scenario {
        Scenario::TxBatchOrdered => tx_batch(&state_init, vc as usize, false),
        Scenario::TxBatchReversed => tx_batch(&state_init, vc as usize, true),
        Scenario::Idle | Scenario::MaxDivergence => Vec::new(),
    };
    let tx_refs: Vec<&[u8]> = txs.iter().map(|tx| tx.as_slice()).collect();

    for _ in 0..cfg.warmup {
        let mut state = state_init;
        advance_epoch(&mut state, &input, &tx_refs).map_err(|e| format!("warmup failed: {e:?}"))?;
        std::hint::black_box(state.state_root);
    }

    let start = Instant::now();
    let mut last_root = [0u8; 32];
    for _ in 0..cfg.iters {
        let mut state = state_init;
        advance_epoch(&mut state, &input, &tx_refs).map_err(|e| format!("run failed: {e:?}"))?;
        last_root = state.state_root;
        std::hint::black_box(last_root);
    }
    let elapsed = start.elapsed().as_secs_f64();
    let epochs_per_second = cfg.iters as f64 / elapsed;
    let tx_per_second = epochs_per_second * scenario.tx_per_epoch(vc) as f64;
    Ok((epochs_per_second, tx_per_second, last_root))
}

fn print_shard_model(base_tps: f64, shards: &[usize]) {
    if base_tps <= 0.0 {
        return;
    }
    print!(" shard_model_tps=[");
    for (i, shard_count) in shards.iter().enumerate() {
        if i > 0 {
            print!(", ");
        }
        print!("{}:{:.2}", shard_count, base_tps * *shard_count as f64);
    }
    print!("]");
}

fn main() {
    let cfg = match parse_config() {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!("error: {err}");
            std::process::exit(2);
        }
    };

    println!("# Domain A TPS smoke model");
    println!(
        "iters={} warmup={} shards={:?}",
        cfg.iters, cfg.warmup, cfg.shards
    );
    println!("note=CPU-only Domain A; shard model is independent-shard linear capacity, not network throughput");

    let scenarios = [
        Scenario::Idle,
        Scenario::MaxDivergence,
        Scenario::TxBatchOrdered,
        Scenario::TxBatchReversed,
    ];

    for &vc in &[1u32, 16, 128, 512, 1024] {
        for scenario in scenarios {
            match run_scenario(vc, scenario, &cfg) {
                Ok((eps, tps, root)) => {
                    print!(
                        "scenario={} validators={} epochs_per_sec={:.2} tx_per_sec={:.2} root_prefix={:02x}{:02x}{:02x}{:02x}",
                        scenario.name(),
                        vc,
                        eps,
                        tps,
                        root[0],
                        root[1],
                        root[2],
                        root[3]
                    );
                    print_shard_model(tps, &cfg.shards);
                    println!();
                }
                Err(err) => {
                    eprintln!(
                        "scenario={} validators={} error={}",
                        scenario.name(),
                        vc,
                        err
                    );
                    std::process::exit(1);
                }
            }
        }
    }
}
