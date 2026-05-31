//! Shared preimage-building and TOML canonicalization for genesis tooling.
//!
//! Both `genesis-hash` and `genesis-cert` call `build_preimage` to construct
//! the identical deterministic byte sequence that feeds `h_cascade`. Any drift
//! between the two tools is caught by the parity test in `genesis_cert.rs`.

use std::{fs, path::Path};

/// Build the genesis hash preimage from the artifact manifest at
/// `<repo_root>/spec/genesis-artifacts.txt`.
///
/// Each artifact is framed as:
///   `path_bytes NUL decimal_len NUL file_bytes NUL`
///
/// `GENESIS_CONSTANTS.toml` is canonicalized via
/// [`canonicalize_genesis_constants`] before framing.
pub fn build_preimage(repo_root: &Path) -> Vec<u8> {
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
        let raw = fs::read(&path).unwrap_or_else(|e| panic!("cannot read artifact {}: {}", rel, e));

        let data: Vec<u8> = if rel == "GENESIS_CONSTANTS.toml" {
            let text = std::str::from_utf8(&raw).expect("GENESIS_CONSTANTS.toml must be UTF-8");
            canonicalize_genesis_constants(text).into_bytes()
        } else {
            raw
        };

        preimage.extend_from_slice(rel.as_bytes());
        preimage.push(0u8);
        preimage.extend_from_slice(data.len().to_string().as_bytes());
        preimage.push(0u8);
        preimage.extend_from_slice(&data);
        preimage.push(0u8);
    }

    preimage
}

/// Canonicalize `GENESIS_CONSTANTS.toml` for inclusion in the genesis hash
/// preimage.
///
/// Replaces self-referential computed values with `<SELF>` so that writing
/// the GRC certificate values back into the file does not perturb the
/// preimage and create a circular dependency.
///
/// Rules:
/// - The `genesis_hash` field is always blanked, regardless of section.
/// - Inside GRC computed sections (`[genesis.certificate]`,
///   `[genesis.hedge_roots]`, `[genesis.entropy]`, `[genesis.supply_chain]`):
///   any field whose quoted string value is a hex string ≥ 64 chars, or
///   starts with a tagged prefix (`ARGON2ID:`, `QASH-ESR:`), is replaced with
///   `<SELF>`.
/// - `[genesis.timestamps]`: only inline hex token hashes ≥ 64 chars inside
///   `rfc3161 = [...]` are blanked; empty arrays and path strings are preserved.
/// - All other sections and fields are copied verbatim.
pub fn canonicalize_genesis_constants(text: &str) -> String {
    const GRC_COMPUTED_SECTIONS: &[&str] = &[
        "[genesis.certificate]",
        "[genesis.hedge_roots]",
        "[genesis.entropy]",
        "[genesis.supply_chain]",
    ];

    let mut result = String::with_capacity(text.len());
    let trailing_newline = text.ends_with('\n');
    let mut in_grc_section = false;
    let mut in_timestamps = false;

    for line in text.lines() {
        let trimmed = line.trim();

        // Update section tracking on section headers.
        if trimmed.starts_with('[') {
            if trimmed.starts_with("[[") {
                // Array table — reset to prevent stale section state.
                in_grc_section = false;
                in_timestamps = false;
            } else {
                in_grc_section = GRC_COMPUTED_SECTIONS.contains(&trimmed);
                in_timestamps = trimmed == "[genesis.timestamps]";
            }
        }

        // Always blank genesis_hash regardless of section.
        if line.trim_start().starts_with("genesis_hash") {
            if let Some(blanked) = blank_tagged_field(line, "QASH-CASCADE-7:<SELF>") {
                result.push_str(&blanked);
                result.push('\n');
                continue;
            }
        }

        // Inside GRC computed sections: blank hex and tagged-prefix values.
        if in_grc_section {
            if let Some(blanked) = try_blank_grc_field(line) {
                result.push_str(&blanked);
                result.push('\n');
                continue;
            }
        }

        // Inside timestamps: blank only populated rfc3161 token hashes.
        if in_timestamps {
            if let Some(blanked) = try_blank_rfc3161_hashes(line) {
                result.push_str(&blanked);
                result.push('\n');
                continue;
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

/// Replace a field's quoted string value with `replacement` unconditionally.
/// Returns `None` if the line doesn't match the expected `key = "..."` shape.
fn blank_tagged_field(line: &str, replacement: &str) -> Option<String> {
    let eq_pos = line.find('=')?;
    let after_eq = line[eq_pos + 1..].trim_start();
    if !after_eq.starts_with('"') {
        return None;
    }
    let prefix = &line[..eq_pos + 1];
    Some(format!("{} \"{}\"", prefix, replacement))
}

/// If `line` is a `key = "value"` line whose value is a computed GRC field
/// (hex ≥ 64 chars, or prefixed with `ARGON2ID:` / `QASH-ESR:`), return the
/// blanked version. Otherwise return `None`.
fn try_blank_grc_field(line: &str) -> Option<String> {
    let eq_pos = line.find('=')?;
    let after_eq = line[eq_pos + 1..].trim_start();
    if !after_eq.starts_with('"') {
        return None;
    }
    let content = extract_first_quoted(after_eq)?;
    if is_computed_value(content) {
        let prefix = &line[..eq_pos + 1];
        Some(format!("{} \"<SELF>\"", prefix))
    } else {
        None
    }
}

/// Blank hex token hashes inside `rfc3161 = [...]` lines.
/// Empty arrays (`rfc3161 = []`) and path strings are preserved.
fn try_blank_rfc3161_hashes(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    // Only act on lines that look like: rfc3161 = [...]
    if !trimmed.starts_with("rfc3161") {
        return None;
    }
    let eq_pos = line.find('=')?;
    let after_eq = line[eq_pos + 1..].trim_start();
    // Empty array — preserve as-is.
    if after_eq.trim() == "[]" || after_eq.trim() == "[ ]" {
        return None;
    }
    // Replace any 64+-char hex strings inside the array with "<SELF>".
    let replaced = replace_hex_in_array(after_eq);
    if replaced == after_eq {
        None
    } else {
        let prefix = &line[..eq_pos + 1];
        Some(format!("{} {}", prefix, replaced))
    }
}

/// Replace hex strings ≥ 64 chars inside a TOML array literal with `"<SELF>"`.
fn replace_hex_in_array(s: &str) -> String {
    // Simple scan: replace each "..." token that is all-hex and ≥ 64 chars.
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '"' {
            let mut token = String::new();
            for inner in chars.by_ref() {
                if inner == '"' {
                    break;
                }
                token.push(inner);
            }
            if token.len() >= 64 && token.chars().all(|c| c.is_ascii_hexdigit()) {
                out.push_str("\"<SELF>\"");
            } else {
                out.push('"');
                out.push_str(&token);
                out.push('"');
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Extract the content of the first `"..."` string token.
fn extract_first_quoted(s: &str) -> Option<&str> {
    let start = s.find('"')? + 1;
    let rest = &s[start..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

/// Returns `true` if a quoted string value should be blanked as a computed GRC field.
fn is_computed_value(s: &str) -> bool {
    // Hex string ≥ 64 chars (128 hex = 64 bytes, 64 hex = 32 bytes)
    (s.len() >= 64 && s.chars().all(|c| c.is_ascii_hexdigit()))
    // Tagged-prefix computed values
    || s.starts_with("ARGON2ID:")
    || s.starts_with("QASH-ESR:")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blanks_genesis_hash() {
        let toml = "genesis_hash = \"QASH-CASCADE-7:deadbeef\"\n";
        let out = canonicalize_genesis_constants(toml);
        assert_eq!(out, "genesis_hash = \"QASH-CASCADE-7:<SELF>\"\n");
    }

    #[test]
    fn blanks_sha3_style_genesis_hash() {
        let toml = "genesis_hash = \"SHA3-256:abc\"\n";
        let out = canonicalize_genesis_constants(toml);
        assert_eq!(out, "genesis_hash = \"QASH-CASCADE-7:<SELF>\"\n");
    }

    #[test]
    fn blanks_hedge_roots_hex() {
        let hex64 = "a".repeat(128);
        let toml = format!("[genesis.hedge_roots]\nsha3_512 = \"{}\"\n", hex64);
        let out = canonicalize_genesis_constants(&toml);
        assert!(out.contains("sha3_512 = \"<SELF>\""), "got: {}", out);
    }

    #[test]
    fn blanks_work_root_argon2id() {
        let toml = "[genesis.certificate]\nwork_root = \"ARGON2ID:cafebabe\"\n";
        let out = canonicalize_genesis_constants(toml);
        assert!(out.contains("work_root = \"<SELF>\""), "got: {}", out);
    }

    #[test]
    fn blanks_supply_chain_hashes() {
        let hex32 = "b".repeat(64);
        let toml = format!(
            "[genesis.supply_chain]\ncompiler_hash = \"{hex32}\"\nrekor_index = 0\ntee_quote = \"not-attested\"\n"
        );
        let out = canonicalize_genesis_constants(&toml);
        assert!(out.contains("compiler_hash = \"<SELF>\""), "got: {}", out);
        // Non-hex fields must be preserved
        assert!(out.contains("rekor_index = 0"));
        assert!(out.contains("tee_quote = \"not-attested\""));
    }

    #[test]
    fn preserves_non_grc_fields() {
        let toml = "[meta]\nversion = \"1.0.0\"\ngenesis_hash = \"QASH-CASCADE-7:ff\"\n\
            [fixed_point]\nscale = 1_000_000\n";
        let out = canonicalize_genesis_constants(toml);
        assert!(out.contains("version = \"1.0.0\""));
        assert!(out.contains("scale = 1_000_000"));
        assert!(out.contains("QASH-CASCADE-7:<SELF>"));
    }

    #[test]
    fn preserves_empty_rfc3161() {
        let toml = "[genesis.timestamps]\nrfc3161 = []\n";
        let out = canonicalize_genesis_constants(toml);
        assert_eq!(out, toml);
    }

    #[test]
    fn blanks_populated_rfc3161_token_hashes() {
        let hex64 = "c".repeat(128);
        let toml = format!("[genesis.timestamps]\nrfc3161 = [\"{hex64}\"]\n");
        let out = canonicalize_genesis_constants(&toml);
        assert!(out.contains("\"<SELF>\""), "got: {}", out);
    }
}
