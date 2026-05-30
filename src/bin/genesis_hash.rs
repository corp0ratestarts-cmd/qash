//! QASH-CASCADE-7 genesis hash computation tool (Domain B).
//!
//! Reads spec/genesis-artifacts.txt, frames each artifact, canonicalizes
//! GENESIS_CONSTANTS.toml, feeds the preimage through h_cascade(), and prints
//! the result as "QASH-CASCADE-7:<128-char hex>".
//!
//! Usage: cargo run --bin genesis-hash -- <repo-root>

use std::{env, fs, path::Path};
use qash_consensus::cascade::h_cascade;

fn main() {
    let repo_root_str = env::args()
        .nth(1)
        .expect("usage: genesis-hash <repo-root>");
    let repo_root = Path::new(&repo_root_str);

    let manifest_path = repo_root.join("spec/genesis-artifacts.txt");
    let manifest = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {}", manifest_path.display(), e));

    let mut preimage: Vec<u8> = Vec::new();

    for raw_line in manifest.lines() {
        let rel = raw_line.trim();
        if rel.is_empty() || rel.starts_with('#') {
            continue;
        }

        let path = repo_root.join(rel);
        let raw = fs::read(&path)
            .unwrap_or_else(|e| panic!("cannot read artifact {}: {}", rel, e));

        let data: Vec<u8> = if rel == "GENESIS_CONSTANTS.toml" {
            let text = std::str::from_utf8(&raw)
                .expect("GENESIS_CONSTANTS.toml must be UTF-8");
            canonicalize_genesis_hash(text).into_bytes()
        } else {
            raw
        };

        // Frame: path_bytes NUL decimal_len NUL file_bytes NUL
        preimage.extend_from_slice(rel.as_bytes());
        preimage.push(0u8);
        preimage.extend_from_slice(data.len().to_string().as_bytes());
        preimage.push(0u8);
        preimage.extend_from_slice(&data);
        preimage.push(0u8);
    }

    let hash = h_cascade(&preimage);
    let hex: String = hash.iter().map(|b| format!("{:02x}", b)).collect();
    println!("QASH-CASCADE-7:{}", hex);
}

/// Replace the genesis_hash field value with the canonical placeholder.
/// Handles both old SHA3-256 and new QASH-CASCADE-7 prefixes, and any
/// whitespace variation around the `=` sign.
fn canonicalize_genesis_hash(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let trailing_newline = text.ends_with('\n');

    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("genesis_hash") {
            if let Some(eq_pos) = line.find('=') {
                let after_eq = line[eq_pos + 1..].trim_start();
                if after_eq.starts_with('"') {
                    // Preserve indentation up to and including the '='
                    let prefix = &line[..eq_pos + 1];
                    result.push_str(prefix);
                    result.push_str(" \"QASH-CASCADE-7:<SELF>\"");
                    result.push('\n');
                    continue;
                }
            }
        }
        result.push_str(line);
        result.push('\n');
    }

    if !trailing_newline && result.ends_with('\n') {
        result.pop();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalizes_sha3_hash() {
        let toml = "genesis_hash = \"SHA3-256:abc123def456\"\n";
        let out = canonicalize_genesis_hash(toml);
        assert_eq!(out, "genesis_hash = \"QASH-CASCADE-7:<SELF>\"\n");
    }

    #[test]
    fn canonicalizes_cascade7_hash() {
        let toml = "genesis_hash = \"QASH-CASCADE-7:deadbeef\"\n";
        let out = canonicalize_genesis_hash(toml);
        assert_eq!(out, "genesis_hash = \"QASH-CASCADE-7:<SELF>\"\n");
    }

    #[test]
    fn canonicalizes_with_surrounding_lines() {
        let toml = "[meta]\nlock_algorithm = \"QASH-CASCADE-7\"\ngenesis_hash = \"QASH-CASCADE-7:ff00\"\ngenesis_status = \"provisional\"\n";
        let out = canonicalize_genesis_hash(toml);
        assert!(out.contains("QASH-CASCADE-7:<SELF>"));
        assert!(out.contains("lock_algorithm = \"QASH-CASCADE-7\""));
        assert!(out.contains("genesis_status = \"provisional\""));
    }
}
