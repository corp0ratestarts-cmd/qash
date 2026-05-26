// MVP demonstrator — deterministic fixture generator.
//
// Prints a stable set of public commitment records and their expected
// commitment root to stdout. Useful for pilot operators to verify that
// their local build produces byte-identical output to the reference.
//
// Usage:
//   cargo run --example mvp_fixtures --features std
//
// The output is deterministic across platforms given the same Rust toolchain.
// All values are derived from the fixed-seed fixtures below; no randomness is used.

fn fixture_bytes_rotate_left(seed: u8) -> [u8; 32] {
    core::array::from_fn(|i| (i as u8).wrapping_add(seed).rotate_left(1))
}

fn fixture_bytes_rotate_right(seed: u8) -> [u8; 32] {
    core::array::from_fn(|i| (i as u8).wrapping_add(seed).rotate_right(1))
}

fn sha3_256_commit(tag: &[u8], input: &[u8]) -> [u8; 32] {
    use sha3::{Digest, Sha3_256};
    let mut h = Sha3_256::new();
    h.update(tag);
    h.update((input.len() as u64).to_le_bytes());
    h.update(input);
    h.finalize().into()
}

fn replay_root_step(prev: [u8; 32], record: &[u8]) -> [u8; 32] {
    use sha3::{Digest, Sha3_256};
    let mut h = Sha3_256::new();
    h.update(b"QASH-MVP-REPLAY-ROOT\0");
    h.update(prev);
    h.update((record.len() as u64).to_le_bytes());
    h.update(record);
    h.finalize().into()
}

fn hex32(b: [u8; 32]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn main() {
    println!("# QASH MVP Deterministic Fixture Pack");
    println!("# Build: cargo run --example mvp_fixtures");
    println!("# All values are fixed-seed, platform-independent, and replay-stable.");
    println!();

    // Fixed-seed fixture inputs (no randomness).
    let fixtures: &[(&str, u64, u8, &[u8])] = &[
        ("alpha-incident", 1, 0, b"synthetic alpha incident body"),
        ("beta-incident", 2, 1, b"synthetic beta incident body"),
        ("gamma-incident", 3, 2, b"synthetic gamma incident body"),
    ];

    let disclosure_commitment = fixture_bytes_rotate_left(0xFF);

    let mut root = [0u8; 32];
    for (label, epoch, seed, body) in fixtures {
        let nonce = fixture_bytes_rotate_right(*seed);
        let payload_commitment = sha3_256_commit(b"QASH-MVP-PAYLOAD-COMMITMENT\0", body);

        // Public export bytes: version(4) + epoch(8) + nonce_commitment(32)
        //                      + payload_commitment(32) + disclosure_key_commitment(32)
        //                      + domain_tag(32) = 140 bytes
        let nonce_commitment = sha3_256_commit(b"QASH-MVP-NONCE-COMMITMENT\0", &nonce);

        let mut record = [0u8; 140];
        record[0..4].copy_from_slice(&1u32.to_le_bytes()); // version = 1
        record[4..12].copy_from_slice(&epoch.to_le_bytes());
        record[12..44].copy_from_slice(&nonce_commitment);
        record[44..76].copy_from_slice(&payload_commitment);
        record[76..108].copy_from_slice(&disclosure_commitment);
        // bytes 108..140 = domain tag (zeros for this fixture)

        root = replay_root_step(root, &record);

        println!("fixture: {label}");
        println!("  epoch:                  {epoch}");
        println!("  nonce_commitment:       {}", hex32(nonce_commitment));
        println!("  payload_commitment:     {}", hex32(payload_commitment));
        println!("  disclosure_commitment:  {}", hex32(disclosure_commitment));
        println!(
            "  record_hex:             {}",
            record
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        );
        println!();
    }

    println!("commitment_root: {}", hex32(root));
    println!();
    println!("# Operators: compare commitment_root with the output of");
    println!("#   qash-demo replay --dir <workspace> --report report.json");
    println!("# after issuing the same three receipts in the same order.");
}
