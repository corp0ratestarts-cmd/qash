//! Benchmark-lite timings for the MVP vault (Domain B demonstrator only).
//!
//! Measures: WAL append throughput, public export generation, and replay
//! latency at representative record counts. These are not production SLAs —
//! they establish a baseline for detecting regressions during TRL 5 hardening.
//!
//! Run with:
//!   cargo bench -p qash-pal --features std --bench mvp_vault

#![cfg(feature = "std")]

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use qash_pal::mvp_vault::MvpReceiptVault;
use std::fs;

fn temp_workspace(label: &str) -> std::path::PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("qash-bench-vault-{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    p
}

fn deterministic_nonce(i: u64) -> [u8; 32] {
    let mut b = [0u8; 32];
    b[0..8].copy_from_slice(&i.to_le_bytes());
    b[8..16].copy_from_slice(&(!i).to_le_bytes());
    b
}

fn disclosure_commitment() -> [u8; 32] {
    [0xDC_u8; 32]
}

// ── WAL append throughput ─────────────────────────────────────────────────

fn bench_issue_receipt(c: &mut Criterion) {
    let mut group = c.benchmark_group("mvp_vault/issue_receipt");
    let path = temp_workspace("issue");
    let vault = MvpReceiptVault::init(&path).unwrap();

    let mut seq: u64 = 0;
    group.bench_function("single", |b| {
        b.iter(|| {
            let body = format!("synthetic incident body {seq}");
            vault
                .issue_receipt(
                    seq,
                    deterministic_nonce(seq),
                    body.as_bytes(),
                    disclosure_commitment(),
                )
                .unwrap();
            seq += 1;
        });
    });

    group.finish();
    let _ = fs::remove_dir_all(&path);
}

// ── Export throughput ─────────────────────────────────────────────────────

fn bench_export_public_commitments(c: &mut Criterion) {
    let mut group = c.benchmark_group("mvp_vault/export_public_commitments");

    for record_count in [10u64, 100, 500] {
        let path = temp_workspace(&format!("export-{record_count}"));
        let vault = MvpReceiptVault::init(&path).unwrap();
        for i in 0..record_count {
            vault
                .issue_receipt(
                    i,
                    deterministic_nonce(i),
                    b"synthetic incident body",
                    disclosure_commitment(),
                )
                .unwrap();
        }

        group.bench_with_input(
            BenchmarkId::from_parameter(record_count),
            &record_count,
            |b, _| {
                b.iter(|| vault.export_public_commitments().unwrap());
            },
        );
        let _ = fs::remove_dir_all(&path);
    }

    group.finish();
}

// ── Replay latency ────────────────────────────────────────────────────────

fn bench_read_all_public_exports(c: &mut Criterion) {
    let mut group = c.benchmark_group("mvp_vault/read_all_public_exports");

    for record_count in [10u64, 100, 500] {
        let path = temp_workspace(&format!("replay-{record_count}"));
        let vault = MvpReceiptVault::init(&path).unwrap();
        for i in 0..record_count {
            vault
                .issue_receipt(
                    i,
                    deterministic_nonce(i),
                    b"synthetic incident body",
                    disclosure_commitment(),
                )
                .unwrap();
        }

        group.bench_with_input(
            BenchmarkId::from_parameter(record_count),
            &record_count,
            |b, _| {
                b.iter(|| vault.read_all_public_exports().unwrap());
            },
        );
        let _ = fs::remove_dir_all(&path);
    }

    group.finish();
}

// ── Import latency ────────────────────────────────────────────────────────

fn bench_import_public_commitments(c: &mut Criterion) {
    let mut group = c.benchmark_group("mvp_vault/import_public_commitments");

    for record_count in [10u64, 100, 500] {
        // Build a public export blob of `record_count` records.
        let src_path = temp_workspace(&format!("import-src-{record_count}"));
        let src_vault = MvpReceiptVault::init(&src_path).unwrap();
        for i in 0..record_count {
            src_vault
                .issue_receipt(
                    i,
                    deterministic_nonce(i),
                    b"synthetic incident body",
                    disclosure_commitment(),
                )
                .unwrap();
        }
        let public = src_vault.export_public_commitments().unwrap();

        let dst_path = temp_workspace(&format!("import-dst-{record_count}"));
        let dst = MvpReceiptVault::init(&dst_path).unwrap();

        group.bench_with_input(
            BenchmarkId::from_parameter(record_count),
            &record_count,
            |b, _| {
                b.iter(|| {
                    dst.import_public_commitments(&public).unwrap()
                });
            },
        );
        let _ = fs::remove_dir_all(&src_path);
        let _ = fs::remove_dir_all(&dst_path);
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_issue_receipt,
    bench_export_public_commitments,
    bench_read_all_public_exports,
    bench_import_public_commitments,
);
criterion_main!(benches);
