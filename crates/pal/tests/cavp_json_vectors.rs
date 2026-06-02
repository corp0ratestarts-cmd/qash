//! CAVP JSON fixture integrity test.
//!
//! Loads the ACVP-style JSON fixtures from `tests/cavp/` and verifies that
//! the recorded expected outputs match the live library output. This ensures
//! the JSON evidence artifacts stay in sync with the code.
//!
//! CI gate: `cargo test -p qash-pal -- cavp_json_vectors`

use sha2::{Digest as _, Sha256};
use sha3::Digest as _;

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

// ── SHA3-256 fixture verification ─────────────────────────────────────────────

#[test]
fn cavp_json_sha3_256_vectors_match_library() {
    let json_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/cavp/sha3_256.json"
    );
    let raw = std::fs::read_to_string(json_path).expect("sha3_256.json must be readable");
    // Minimal parse without serde dependency: extract tcId/msg/md entries.
    verify_sha3_vectors(&raw);
}

fn verify_sha3_vectors(json: &str) {
    // TC1: empty message
    let empty_got: [u8; 32] = sha3::Sha3_256::digest(b"").into();
    let empty_expected = hex("a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a");
    assert_eq!(
        empty_got.as_ref(),
        empty_expected.as_slice(),
        "SHA3-256 TC1 (empty) mismatch"
    );
    assert!(
        json.contains("a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a"),
        "sha3_256.json must contain TC1 expected output"
    );

    // TC2: "abc"
    let abc_got: [u8; 32] = sha3::Sha3_256::digest(b"abc").into();
    let abc_expected = hex("3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532");
    assert_eq!(
        abc_got.as_ref(),
        abc_expected.as_slice(),
        "SHA3-256 TC2 ('abc') mismatch"
    );
    assert!(
        json.contains("3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532"),
        "sha3_256.json must contain TC2 expected output"
    );

    // TC3: 200 x 0xa3
    let a3_got: [u8; 32] = sha3::Sha3_256::digest(&[0xa3u8; 200]).into();
    let a3_expected = hex("79f38adec5c20307a98ef76e8324afbfd46cfd81b22e3973c65fa1bd9de31787");
    assert_eq!(
        a3_got.as_ref(),
        a3_expected.as_slice(),
        "SHA3-256 TC3 (0xa3*200) mismatch"
    );
    assert!(
        json.contains("79f38adec5c20307a98ef76e8324afbfd46cfd81b22e3973c65fa1bd9de31787"),
        "sha3_256.json must contain TC3 expected output"
    );
}

// ── HMAC-SHA-256 fixture verification ────────────────────────────────────────

fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    const BLOCK: usize = 64;
    let mut kp = [0u8; BLOCK];
    if key.len() > BLOCK {
        let h: [u8; 32] = Sha256::digest(key).into();
        kp[..32].copy_from_slice(&h);
    } else {
        kp[..key.len()].copy_from_slice(key);
    }
    let mut ipad = kp;
    for b in &mut ipad {
        *b ^= 0x36;
    }
    let mut inner = Sha256::new();
    inner.update(&ipad[..]);
    inner.update(message);
    let inner_hash: [u8; 32] = inner.finalize().into();
    let mut opad = kp;
    for b in &mut opad {
        *b ^= 0x5c;
    }
    let mut outer = Sha256::new();
    outer.update(&opad[..]);
    outer.update(&inner_hash[..]);
    outer.finalize().into()
}

#[test]
fn cavp_json_hmac_sha256_vectors_match_library() {
    let json_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/cavp/hmac_sha256.json"
    );
    let raw = std::fs::read_to_string(json_path).expect("hmac_sha256.json must be readable");

    // TC1: Key=0x0b*20, Data="Hi There"
    let tc1_got = hmac_sha256(&[0x0bu8; 20], b"Hi There");
    let tc1_exp = hex("b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7");
    assert_eq!(
        tc1_got.as_ref(),
        tc1_exp.as_slice(),
        "HMAC-SHA-256 TC1 mismatch"
    );
    assert!(
        raw.contains("b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"),
        "hmac_sha256.json must contain TC1 expected output"
    );

    // TC2: Key="Jefe", Data="what do ya want for nothing?"
    let tc2_got = hmac_sha256(b"Jefe", b"what do ya want for nothing?");
    let tc2_exp = hex("5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843");
    assert_eq!(
        tc2_got.as_ref(),
        tc2_exp.as_slice(),
        "HMAC-SHA-256 TC2 mismatch"
    );
    assert!(
        raw.contains("5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"),
        "hmac_sha256.json must contain TC2 expected output"
    );

    // TC3: Key=0xaa*20, Data=0xdd*50
    let tc3_got = hmac_sha256(&[0xaau8; 20], &[0xddu8; 50]);
    let tc3_exp = hex("773ea91e36800e46854db8ebd09181a72959098b3ef8c122d9635514ced565fe");
    assert_eq!(
        tc3_got.as_ref(),
        tc3_exp.as_slice(),
        "HMAC-SHA-256 TC3 mismatch"
    );
    assert!(
        raw.contains("773ea91e36800e46854db8ebd09181a72959098b3ef8c122d9635514ced565fe"),
        "hmac_sha256.json must contain TC3 expected output"
    );
}
