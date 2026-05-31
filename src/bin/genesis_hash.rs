//! QASH-CASCADE-7 genesis hash computation tool (Domain B).
//!
//! Reads spec/genesis-artifacts.txt, frames each artifact, canonicalizes
//! GENESIS_CONSTANTS.toml, feeds the preimage through h_cascade(), and prints
//! the result as "QASH-CASCADE-7:<128-char hex>".
//!
//! Usage: cargo run --bin genesis-hash -- <repo-root>

use qash::genesis_preimage::build_preimage;
use qash_consensus::cascade::h_cascade;
use std::{env, path::Path};

fn main() {
    let repo_root_str = env::args().nth(1).expect("usage: genesis-hash <repo-root>");
    let repo_root = Path::new(&repo_root_str);

    let preimage = build_preimage(repo_root);
    let hash = h_cascade(&preimage);
    let hex: String = hash.iter().map(|b| format!("{:02x}", b)).collect();
    println!("QASH-CASCADE-7:{}", hex);
}

#[cfg(test)]
mod tests {
    use qash::genesis_preimage::canonicalize_genesis_constants;

    #[test]
    fn canonicalizes_sha3_hash() {
        let toml = "genesis_hash = \"SHA3-256:abc123def456\"\n";
        let out = canonicalize_genesis_constants(toml);
        assert_eq!(out, "genesis_hash = \"QASH-CASCADE-7:<SELF>\"\n");
    }

    #[test]
    fn canonicalizes_cascade7_hash() {
        let toml = "genesis_hash = \"QASH-CASCADE-7:deadbeef\"\n";
        let out = canonicalize_genesis_constants(toml);
        assert_eq!(out, "genesis_hash = \"QASH-CASCADE-7:<SELF>\"\n");
    }

    #[test]
    fn canonicalizes_with_surrounding_lines() {
        let toml = "[meta]\nlock_algorithm = \"QASH-CASCADE-7\"\ngenesis_hash = \"QASH-CASCADE-7:ff00\"\ngenesis_status = \"provisional\"\n";
        let out = canonicalize_genesis_constants(toml);
        assert!(out.contains("QASH-CASCADE-7:<SELF>"));
        assert!(out.contains("lock_algorithm = \"QASH-CASCADE-7\""));
        assert!(out.contains("genesis_status = \"provisional\""));
    }
}
