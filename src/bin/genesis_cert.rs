//! QASH-GRC-7-7-v2 Genesis Ratchet Certificate generator (Domain B).
//!
//! Reads the same preimage as `genesis-hash`, then computes:
//!   - 7 hedge roots (L1 primitive outputs via h_cascade_l1_primitives)
//!   - work_root via Argon2id (p=1, 512 MiB, t=3) — single-lane memory-hard
//!     anti-grinding. This is NOT a VDF or proof of sequential work.
//!   - supply-chain hashes (compiler, rust-toolchain, Cargo.lock)
//!
//! Outputs TOML sections to stdout for manual review and pasting into
//! GENESIS_CONSTANTS.toml. After pasting, re-run `genesis-hash` to obtain
//! the updated genesis_hash.
//!
//! Usage: cargo run --bin genesis-cert -- <repo-root>

use argon2::{Algorithm, Argon2, Params, Version};
use qash::genesis_preimage::build_preimage;
use qash_consensus::cascade::{h_cascade, h_cascade_l1_primitives};
use sha3::{Digest, Sha3_256, Sha3_512};
use std::{env, path::Path, process::Command};

fn main() {
    let repo_root_str = env::args().nth(1).expect("usage: genesis-cert <repo-root>");
    let repo_root = Path::new(&repo_root_str);

    // Build the canonical preimage (identical to genesis-hash).
    let preimage = build_preimage(repo_root);
    let genesis_hash_bytes = h_cascade(&preimage);
    let genesis_hex: String = genesis_hash_bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();

    // Seven L1 hedge roots.
    let primitives = h_cascade_l1_primitives(&preimage);
    let [sha3_512, blake3_xof_64, k12_xof_64, sm3_double, streebog_512, kupyna_512, lsh_512] =
        primitives;

    // Provisional entropy salt: SHA3-512("QASH:GRC:PROVISIONAL:ENTROPY" || genesis_hash_bytes).
    // At lock time this is replaced with SHA3-512(locked_public_entropy || genesis_hash_bytes).
    let provisional_salt = {
        let mut h = Sha3_512::new();
        h.update(b"QASH:GRC:PROVISIONAL:ENTROPY");
        h.update(genesis_hash_bytes);
        let full: [u8; 64] = h.finalize().into();
        full
    };

    // Argon2id work root — p=1 for single-lane sequential memory-hardness.
    // Use all 64 bytes of the SHA3-512 salt output; no truncation.
    let work_root = compute_work_root(&genesis_hash_bytes, &provisional_salt);

    // Supply-chain hashes.
    let compiler_hash = sha3_256_hex(&compiler_version_bytes());
    let toolchain_hash = sha3_256_hex(
        &std::fs::read(repo_root.join("rust-toolchain.toml"))
            .expect("cannot read rust-toolchain.toml"),
    );
    let cargo_lock_hash =
        sha3_256_hex(&std::fs::read(repo_root.join("Cargo.lock")).expect("cannot read Cargo.lock"));

    // Print TOML sections.
    println!("# artifact_identity_root: QASH-CASCADE-7:{genesis_hex}");
    println!();
    println!("# ---------------------------------------------------------------------------");
    println!("# GRC-7-7-v2 Genesis Ratchet Certificate");
    println!("# ---------------------------------------------------------------------------");
    println!();
    println!("[genesis.certificate]");
    println!("algorithm = \"QASH-GRC-7-7-v2\"");
    println!("# artifact_identity_root == genesis_hash (QASH-CASCADE-7 output); see [meta].");
    println!("work_root = \"ARGON2ID:{}\"", hex(&work_root));
    println!();
    println!("[genesis.work]");
    println!("algorithm = \"Argon2id\"");
    println!("memory_mib = 512");
    println!("time_cost = 3");
    println!("parallelism = 1");
    println!("output_bytes = 64");
    println!("salt_algorithm = \"SHA3-512\"");
    println!("salt_inputs = [\"QASH:GRC:PROVISIONAL:ENTROPY\", \"genesis_hash_bytes\"]");
    println!("salt_status = \"provisional\"");
    println!(
        "# At lock time: salt = full 64-byte SHA3-512(locked_public_entropy || genesis_hash_bytes)"
    );
    println!();
    println!("[genesis.long_work]");
    println!("enabled = false");
    println!("profile = \"archival-only\"");
    println!("# Optional ESR profile; not required for normal verification.");
    println!();
    println!("[genesis.hedge_policy]");
    println!("total_roots = 7");
    println!("verification_rule = \"all_7_roots_must_match\"");
    println!("security_model = \"last_unbroken_root\"");
    println!("retirement_allowed = false");
    println!("deprecation_status_is_informational = true");
    println!();
    println!("[genesis.hedge_roots]");
    println!("sha3_512          = \"{}\"", hex(&sha3_512));
    println!("blake3_xof_64     = \"{}\"", hex(&blake3_xof_64));
    println!("kangaroo12_xof_64 = \"{}\"", hex(&k12_xof_64));
    println!("sm3_double_width  = \"{}\"", hex(&sm3_double));
    println!("streebog_512      = \"{}\"", hex(&streebog_512));
    println!("kupyna_512        = \"{}\"", hex(&kupyna_512));
    println!("lsh_512           = \"{}\"", hex(&lsh_512));
    println!();
    println!("[genesis.hedge_status]");
    println!("sha3_512          = \"active\"");
    println!("blake3_xof_64     = \"active\"");
    println!("kangaroo12_xof_64 = \"active\"");
    println!("sm3_double_width  = \"active\"");
    println!("streebog_512      = \"active\"");
    println!("kupyna_512        = \"active\"");
    println!("lsh_512           = \"active\"");
    println!("# Informational only. Does not affect verification_rule.");
    println!();
    println!("[genesis.entropy]");
    println!("algorithm = \"QASH-ENTROPY-MIX-v1\"");
    println!("sources_required = 2");
    println!("sources = [\"drand\", \"nist_beacon\", \"bitcoin_block_optional\"]");
    println!("entropy_rule = \"SHA3-512(drand_randomness || nist_beacon_value || bitcoin_block_hash_if_present || genesis_hash_bytes)\"");
    println!("status = \"provisional\"");
    println!("drand_round = 0");
    println!("nist_beacon_round = \"\"");
    println!("bitcoin_block_height = 0");
    println!("bitcoin_confirmations_required = 12");
    println!();
    println!("[genesis.supply_chain]");
    println!("compiler_hash       = \"{compiler_hash}\"");
    println!("rust_toolchain_hash = \"{toolchain_hash}\"");
    println!("cargo_lock_hash     = \"{cargo_lock_hash}\"");
    println!("rekor_index = 0");
    println!("tee_quote = \"not-attested\"");
    println!();
    println!("[genesis.timestamps]");
    println!("rfc3161 = []");
}

/// Compute the Argon2id work root.
/// Production params: m=512 MiB, t=3, p=1.
fn compute_work_root(password: &[u8], salt: &[u8]) -> [u8; 64] {
    let params = argon2_production_params();
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = [0u8; 64];
    argon2
        .hash_password_into(password, salt, &mut out)
        .expect("Argon2id computation failed");
    out
}

fn argon2_production_params() -> Params {
    Params::new(
        524_288, // m_cost: 512 MiB in KiB
        3,       // t_cost
        1,       // p_cost: 1 thread — single-lane, not a VDF
        Some(64),
    )
    .expect("invalid Argon2id params")
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn sha3_256_hex(data: &[u8]) -> String {
    let out: [u8; 32] = Sha3_256::digest(data).into();
    hex(&out)
}

fn compiler_version_bytes() -> Vec<u8> {
    let output = Command::new("rustc")
        .args(["--version", "--verbose"])
        .output()
        .expect("cannot run rustc --version --verbose");
    output.stdout
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Real parity fixture test: build_preimage() canonicalizes GRC computed
    /// fields to <SELF> before hashing, so genesis-hash and genesis-cert both
    /// produce the same artifact_identity_root regardless of what values are
    /// written into [genesis.certificate], [genesis.hedge_roots], etc.
    ///
    /// Builds two filesystem fixtures with different GRC field values and
    /// asserts the preimage and final hash are identical.
    #[test]
    fn preimage_parity_grc_fields_are_canonicalized() {
        use std::fs;
        use std::path::PathBuf;

        let tmp: PathBuf = std::env::temp_dir().join(format!("qash-parity-{}", std::process::id()));
        fs::create_dir_all(tmp.join("spec")).unwrap();

        // Manifest lists only GENESIS_CONSTANTS.toml.
        fs::write(
            tmp.join("spec/genesis-artifacts.txt"),
            "GENESIS_CONSTANTS.toml\n",
        )
        .unwrap();

        // Two GENESIS_CONSTANTS.toml files with identical non-GRC content but
        // different values in every GRC computed section.
        let make_toml = |suffix: &str| {
            let h128 = suffix.repeat(128 / suffix.len() + 1)[..128].to_string();
            let h64 = suffix.repeat(64 / suffix.len() + 1)[..64].to_string();
            format!(
                "[meta]\n\
                 lock_algorithm = \"QASH-CASCADE-7\"\n\
                 genesis_hash = \"QASH-CASCADE-7:{h64}\"\n\
                 genesis_status = \"provisional\"\n\
                 \n\
                 [fixed_point]\n\
                 scale = 1000000\n\
                 \n\
                 [genesis.certificate]\n\
                 algorithm = \"QASH-GRC-7-7-v2\"\n\
                 work_root = \"ARGON2ID:{h128}\"\n\
                 \n\
                 [genesis.hedge_roots]\n\
                 sha3_512 = \"{h128}\"\n\
                 blake3_xof_64 = \"{h128}\"\n\
                 \n\
                 [genesis.supply_chain]\n\
                 compiler_hash = \"{h64}\"\n\
                 rekor_index = 0\n\
                 tee_quote = \"not-attested\"\n\
                 \n\
                 [genesis.timestamps]\n\
                 rfc3161 = []\n"
            )
        };

        // Fixture A: GRC fields filled with 'a'/'0' hex chars.
        fs::write(tmp.join("GENESIS_CONSTANTS.toml"), make_toml("a0")).unwrap();
        let preimage_a = build_preimage(&tmp);
        let hash_a = h_cascade(&preimage_a);

        // Fixture B: GRC fields filled with different 'f'/'e' hex chars.
        fs::write(tmp.join("GENESIS_CONSTANTS.toml"), make_toml("fe")).unwrap();
        let preimage_b = build_preimage(&tmp);
        let hash_b = h_cascade(&preimage_b);

        fs::remove_dir_all(&tmp).ok();

        assert_eq!(
            preimage_a, preimage_b,
            "build_preimage() must produce identical bytes regardless of GRC computed field values"
        );
        assert_eq!(
            hash_a, hash_b,
            "artifact_identity_root must be stable despite different GRC field values"
        );
        assert_ne!(hash_a, [0u8; 64]);
    }

    /// Confirm h_cascade_l1_primitives returns 7 distinct 64-byte outputs.
    #[test]
    fn l1_primitives_are_distinct() {
        let input = b"QASH:GRC:L1:PARITY:TEST";
        let roots = h_cascade_l1_primitives(input);
        for i in 0..7 {
            assert_ne!(roots[i], [0u8; 64], "root {i} must be non-zero");
            for j in (i + 1)..7 {
                assert_ne!(roots[i], roots[j], "roots {i} and {j} must differ");
            }
        }
    }

    /// Argon2id with test params produces a non-zero 64-byte output.
    /// Uses small params to keep CI fast; production uses m=512 MiB.
    #[test]
    fn argon2_test_params_produce_output() {
        let params = Params::new(8_192, 1, 1, Some(64)).expect("invalid test params");
        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
        let password = b"test-genesis-hash-bytes";
        let salt = b"test-provisional-salt-32bytes!!!";
        let mut out = [0u8; 64];
        argon2
            .hash_password_into(password, salt, &mut out)
            .expect("Argon2id test failed");
        assert_ne!(out, [0u8; 64]);
    }

    /// Provisional salt depends on the genesis hash bytes.
    #[test]
    fn provisional_salt_depends_on_genesis_hash() {
        let gh1 = [0x01u8; 64];
        let gh2 = [0x02u8; 64];
        let salt1 = {
            let mut h = Sha3_512::new();
            h.update(b"QASH:GRC:PROVISIONAL:ENTROPY");
            h.update(&gh1);
            let out: [u8; 64] = h.finalize().into();
            out
        };
        let salt2 = {
            let mut h = Sha3_512::new();
            h.update(b"QASH:GRC:PROVISIONAL:ENTROPY");
            h.update(&gh2);
            let out: [u8; 64] = h.finalize().into();
            out
        };
        assert_ne!(salt1, salt2);
    }
}
