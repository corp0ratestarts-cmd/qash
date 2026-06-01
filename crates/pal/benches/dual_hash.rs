use criterion::{criterion_group, criterion_main, Criterion};
use qash_pal::crypto::dual_hash::{allof_hash_pair_32, dual_hash_32};

fn bench_dual_hash(c: &mut Criterion) {
    // Use a u64 epoch value converted to bytes — mirrors real production usage.
    let epoch: u64 = 1;
    let salt = epoch.to_le_bytes();

    for size in [64usize, 1024, 65536] {
        let data = vec![0x42u8; size];

        let name = format!("dual_hash_32/{size}");
        c.bench_function(&name, |b| {
            b.iter(|| dual_hash_32(b"bench", &salt, &data))
        });

        let name = format!("allof_hash_pair_32/{size}");
        c.bench_function(&name, |b| {
            b.iter(|| allof_hash_pair_32(b"bench", &salt, &data))
        });
    }

    let mut manifest_data = Vec::with_capacity(1000 * 32);
    for i in 0..1000u32 {
        let mut h = [0u8; 32];
        h[..4].copy_from_slice(&i.to_le_bytes());
        manifest_data.extend_from_slice(&h);
    }

    c.bench_function("allof_manifest_root/1000_chunk_hashes", |b| {
        b.iter(|| allof_hash_pair_32(b"qash-bench-manifest", &salt, &manifest_data))
    });
}

criterion_group!(benches, bench_dual_hash);
criterion_main!(benches);
