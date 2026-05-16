//! QASH address encoding — Domain B utility crate.
//!
//! Converts the 48-byte validator leaf index (produced by `qash_consensus::derive`)
//! into a human-readable Bech32m string with F4Jumble permutation applied before
//! encoding, and reverses the process for decoding.
//!
//! # Address anatomy
//! ```text
//! qash1 <80-char bech32m data+checksum>
//! ──────┬───────────────────────────────
//!       │
//!       └── F4Jumble(48-byte leaf index) → base-32 encoded + 6-char checksum
//! ```
//!
//! # F4Jumble (Option A — 3-round Feistel)
//! F4Jumble is a wide-block permutation that ensures a 1-bit error in the raw
//! payload propagates across the entire encoded string, making typos detectable
//! beyond what the Bech32m checksum alone provides.
//!
//! The 48-byte payload is split: left (L) = bytes 0..32, right (R) = bytes 32..48.
//! ```text
//! Round 1: L' = L ⊕ SHA3-256("F4J:R1" ∥ R)
//! Round 2: R' = R ⊕ SHA3-256("F4J:R2" ∥ L')[..16]
//! Round 3: L''= L'⊕ SHA3-256("F4J:R3" ∥ R')
//! ```
//! The permutation is its own inverse: applying F4Jumble twice returns the
//! original payload (proved by the Feistel structure and XOR involution).
//!
//! # Bech32m
//! Implemented inline without external crate dependency. Uses the Bech32m
//! constant M = 0x2bc830a3 (BIP 350). HRP is always `"qash"`.
//!
//! # Domain B compliance
//! This crate is Domain B: it may use std, performs no consensus arithmetic,
//! and never flows into Domain A state. The sole input (leaf index) is produced
//! by Domain A `derive_leaf_index` and treated as opaque bytes here.

use sha3::{Digest, Sha3_256};

// ---------------------------------------------------------------------------
// F4Jumble — 4-round wide-block permutation on 48 bytes (Option A)
//
// Split: L = payload[0..32], R = payload[32..48] (|L|=32, |R|=16)
//
// Forward (encode):
//   a = R ⊕ H("F4J:0" ∥ L)[..16]
//   b = L ⊕ H("F4J:1" ∥ a)
//   c = a ⊕ H("F4J:2" ∥ b)[..16]
//   d = b ⊕ H("F4J:3" ∥ c)
//   output = d ∥ c               (L'=d, R'=c)
//
// Inverse (decode) — reverse the round order:
//   b = d ⊕ H("F4J:3" ∥ c)
//   a = c ⊕ H("F4J:2" ∥ b)[..16]
//   L = b ⊕ H("F4J:1" ∥ a)
//   R = a ⊕ H("F4J:0" ∥ L)[..16]
//   output = L ∥ R
//
// The construction guarantees decode(encode(x)) = x because each decoding
// step exactly undoes the corresponding encoding step (XOR is self-inverse,
// and the hash inputs are recovered in reverse order).
// ---------------------------------------------------------------------------

/// Apply the F4Jumble permutation (encoding direction) in-place.
/// Call `f4jumble_inv` to invert.
pub fn f4jumble(payload: &mut [u8; 48]) {
    let l: [u8; 32] = payload[..32].try_into().unwrap();
    let r: [u8; 16] = payload[32..].try_into().unwrap();

    let a = xor16(r, &h_f4j_16(b"F4J:0", &l));
    let b = xor32(l, &h_f4j_32(b"F4J:1", &a));
    let c = xor16(a, &h_f4j_16(b"F4J:2", &b));
    let d = xor32(b, &h_f4j_32(b"F4J:3", &c));

    payload[..32].copy_from_slice(&d);
    payload[32..].copy_from_slice(&c);
}

/// Invert the F4Jumble permutation (decoding direction) in-place.
pub fn f4jumble_inv(payload: &mut [u8; 48]) {
    let d: [u8; 32] = payload[..32].try_into().unwrap();
    let c: [u8; 16] = payload[32..].try_into().unwrap();

    let b = xor32(d, &h_f4j_32(b"F4J:3", &c));
    let a = xor16(c, &h_f4j_16(b"F4J:2", &b));
    let l = xor32(b, &h_f4j_32(b"F4J:1", &a));
    let r = xor16(a, &h_f4j_16(b"F4J:0", &l));

    payload[..32].copy_from_slice(&l);
    payload[32..].copy_from_slice(&r);
}

fn h_f4j_32(tag: &[u8], data: &[u8]) -> [u8; 32] {
    let mut h = Sha3_256::new();
    h.update(tag);
    h.update(data);
    let out = h.finalize();
    let mut r = [0u8; 32];
    r.copy_from_slice(&out);
    r
}

fn h_f4j_16(tag: &[u8], data: &[u8]) -> [u8; 16] {
    let full = h_f4j_32(tag, data);
    let mut r = [0u8; 16];
    r.copy_from_slice(&full[..16]);
    r
}

fn xor32(mut a: [u8; 32], b: &[u8; 32]) -> [u8; 32] {
    for i in 0..32 { a[i] ^= b[i]; }
    a
}

fn xor16(mut a: [u8; 16], b: &[u8; 16]) -> [u8; 16] {
    for i in 0..16 { a[i] ^= b[i]; }
    a
}

// ---------------------------------------------------------------------------
// Bech32m — inline implementation (HRP = "qash", M = 0x2bc830a3)
// ---------------------------------------------------------------------------

const BECH32M_CONST: u32 = 0x2bc830a3;
const CHARSET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

/// Encode a 48-byte leaf index as a Bech32m address string.
///
/// Returns a string of the form `qash1<80 chars>`.
/// Total length: 4 (hrp) + 1 (separator) + 77 (data) + 6 (checksum) = 88 chars.
pub fn encode(leaf_index: &[u8; 48]) -> String {
    let mut payload = *leaf_index;
    f4jumble(&mut payload);

    // Convert 8-bit groups to 5-bit groups (48 bytes → 77 five-bit values).
    let data5 = to_base32(&payload);

    let hrp = b"qash";
    let checksum = bech32m_checksum(hrp, &data5);

    let mut out = String::with_capacity(88);
    out.push_str("qash");
    out.push('1');
    for b in &data5 {
        out.push(CHARSET[*b as usize] as char);
    }
    for b in &checksum {
        out.push(CHARSET[*b as usize] as char);
    }
    out
}

/// Decode a Bech32m address string back to the 48-byte leaf index.
///
/// Returns `Err` on any of: invalid HRP, invalid checksum, wrong data length,
/// or invalid Bech32m characters.
pub fn decode(s: &str) -> Result<[u8; 48], AddressError> {
    let s_lower = s.to_lowercase();
    let s_bytes = s_lower.as_bytes();

    // Find separator '1'.
    let sep = s_bytes
        .iter()
        .rposition(|&b| b == b'1')
        .ok_or(AddressError::NoSeparator)?;

    let hrp = &s_bytes[..sep];
    if hrp != b"qash" {
        return Err(AddressError::BadHrp);
    }

    let data_chars = &s_bytes[sep + 1..];
    if data_chars.len() != 83 {
        // 77 data + 6 checksum
        return Err(AddressError::BadLength);
    }

    // Decode characters to 5-bit values.
    let mut values = [0u8; 83];
    for (i, &ch) in data_chars.iter().enumerate() {
        let v = charset_pos(ch).ok_or(AddressError::InvalidChar(ch as char))?;
        values[i] = v;
    }

    // Verify checksum.
    if !verify_bech32m_checksum(hrp, &values) {
        return Err(AddressError::BadChecksum);
    }

    // Convert 5-bit groups back to 8-bit bytes (first 77 values → 48 bytes).
    let payload_bytes = from_base32(&values[..77])?;
    let mut arr = [0u8; 48];
    arr.copy_from_slice(&payload_bytes);

    // Invert F4Jumble to recover the original leaf index.
    f4jumble_inv(&mut arr);
    Ok(arr)
}

// ---------------------------------------------------------------------------
// Bech32m internal helpers
// ---------------------------------------------------------------------------

/// Convert bytes to 5-bit base-32 groups (big-endian bit packing).
/// 48 bytes × 8 bits = 384 bits → 77 five-bit groups (385 bits, 1 pad bit).
fn to_base32(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(77);
    let mut acc: u32 = 0;
    let mut bits: u32 = 0;
    for &byte in data {
        acc = (acc << 8) | u32::from(byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(((acc >> bits) & 0x1f) as u8);
        }
    }
    if bits > 0 {
        out.push(((acc << (5 - bits)) & 0x1f) as u8);
    }
    out
}

/// Convert 5-bit groups back to bytes.
fn from_base32(data: &[u8]) -> Result<Vec<u8>, AddressError> {
    let mut out = Vec::with_capacity(48);
    let mut acc: u32 = 0;
    let mut bits: u32 = 0;
    for &b in data {
        if b > 31 {
            return Err(AddressError::InvalidChar('?'));
        }
        acc = (acc << 5) | u32::from(b);
        bits += 5;
        while bits >= 8 {
            bits -= 8;
            out.push(((acc >> bits) & 0xff) as u8);
        }
    }
    // Remaining padding bits must be zero.
    if bits > 0 && (acc & ((1 << bits) - 1)) != 0 {
        return Err(AddressError::BadChecksum);
    }
    Ok(out)
}

fn charset_pos(ch: u8) -> Option<u8> {
    CHARSET.iter().position(|&c| c == ch).map(|p| p as u8)
}

fn bech32m_polymod(values: &[u8]) -> u32 {
    const GEN: [u32; 5] = [0x3b6a57b2, 0x26508e6d, 0x1ea119fa, 0x3d4233dd, 0x2a1462b3];
    let mut chk: u32 = 1;
    for &v in values {
        let top = chk >> 25;
        chk = ((chk & 0x1ff_ffff) << 5) ^ u32::from(v);
        for (i, &g) in GEN.iter().enumerate() {
            if (top >> i) & 1 != 0 {
                chk ^= g;
            }
        }
    }
    chk
}

fn hrp_expand(hrp: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(hrp.len() * 2 + 1);
    for &c in hrp {
        v.push(c >> 5);
    }
    v.push(0);
    for &c in hrp {
        v.push(c & 0x1f);
    }
    v
}

fn bech32m_checksum(hrp: &[u8], data: &[u8]) -> [u8; 6] {
    let mut values = hrp_expand(hrp);
    values.extend_from_slice(data);
    values.extend_from_slice(&[0u8; 6]);
    let pm = bech32m_polymod(&values) ^ BECH32M_CONST;
    let mut cs = [0u8; 6];
    for i in 0..6 {
        cs[i] = ((pm >> (5 * (5 - i))) & 0x1f) as u8;
    }
    cs
}

fn verify_bech32m_checksum(hrp: &[u8], data: &[u8]) -> bool {
    let mut values = hrp_expand(hrp);
    values.extend_from_slice(data);
    bech32m_polymod(&values) == BECH32M_CONST
}

// ---------------------------------------------------------------------------
// AddressError
// ---------------------------------------------------------------------------

/// Errors returned by `decode`.
#[derive(Debug, PartialEq, Eq)]
pub enum AddressError {
    /// No '1' separator character found.
    NoSeparator,
    /// HRP is not "qash".
    BadHrp,
    /// Encoded length does not correspond to a 48-byte payload.
    BadLength,
    /// A character is not in the Bech32 charset.
    InvalidChar(char),
    /// Bech32m checksum verification failed.
    BadChecksum,
}

impl core::fmt::Display for AddressError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AddressError::NoSeparator => write!(f, "missing bech32 separator '1'"),
            AddressError::BadHrp => write!(f, "HRP must be 'qash'"),
            AddressError::BadLength => write!(f, "encoded data has wrong length for 48-byte payload"),
            AddressError::InvalidChar(c) => write!(f, "invalid bech32 character: {:?}", c),
            AddressError::BadChecksum => write!(f, "bech32m checksum mismatch"),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const ZERO_LEAF: [u8; 48] = [0u8; 48];

    /// F4Jumble encode → decode roundtrip recovers the original payload.
    #[test]
    fn f4jumble_encode_decode_roundtrip() {
        let mut p = [0u8; 48];
        for (i, b) in p.iter_mut().enumerate() {
            *b = i as u8;
        }
        let original = p;
        f4jumble(&mut p);
        f4jumble_inv(&mut p);
        assert_eq!(p, original, "f4jumble_inv(f4jumble(x)) must equal x");
    }

    /// F4Jumble changes the payload (it is not the identity).
    #[test]
    fn f4jumble_is_not_identity() {
        let mut p = ZERO_LEAF;
        f4jumble(&mut p);
        assert_ne!(p, ZERO_LEAF);
    }

    /// F4Jumble inverse changes the payload (it is not the identity).
    #[test]
    fn f4jumble_inv_is_not_identity() {
        let mut p = ZERO_LEAF;
        f4jumble_inv(&mut p);
        assert_ne!(p, ZERO_LEAF);
    }

    /// Encode produces a string starting with "qash1" and of length 88.
    #[test]
    fn encode_format() {
        let s = encode(&ZERO_LEAF);
        assert!(s.starts_with("qash1"), "address must start with 'qash1'");
        assert_eq!(s.len(), 88, "address must be 88 chars, got {}", s.len());
    }

    /// Round-trip: encode → decode returns the original leaf index.
    #[test]
    fn encode_decode_roundtrip() {
        let mut leaf = [0u8; 48];
        for (i, b) in leaf.iter_mut().enumerate() {
            *b = (i * 7 + 3) as u8;
        }
        let s = encode(&leaf);
        let decoded = decode(&s).expect("decode must succeed");
        assert_eq!(decoded, leaf, "decode(encode(leaf)) must equal leaf");
    }

    /// Zero leaf encodes and decodes correctly.
    #[test]
    fn zero_leaf_roundtrip() {
        let s = encode(&ZERO_LEAF);
        let decoded = decode(&s).expect("decode zero leaf");
        assert_eq!(decoded, ZERO_LEAF);
    }

    /// Two different leaf indices produce different addresses.
    #[test]
    fn different_leaves_produce_different_addresses() {
        let leaf1 = ZERO_LEAF;
        let mut leaf2 = ZERO_LEAF;
        leaf2[0] = 1;
        assert_ne!(encode(&leaf1), encode(&leaf2));
    }

    /// A tampered address character is rejected.
    #[test]
    fn tampered_address_rejected() {
        let s = encode(&ZERO_LEAF);
        let mut bytes = s.into_bytes();
        // Flip one character in the data section (after "qash1").
        bytes[10] = if bytes[10] == b'q' { b'p' } else { b'q' };
        let s2 = String::from_utf8(bytes).unwrap();
        assert!(decode(&s2).is_err(), "tampered address must fail to decode");
    }

    /// Wrong HRP is rejected.
    #[test]
    fn wrong_hrp_rejected() {
        let s = encode(&ZERO_LEAF);
        let bad = "btc1".to_string() + &s[5..];
        assert_eq!(decode(&bad), Err(AddressError::BadHrp));
    }

    /// All characters in a valid address are lowercase Bech32 charset chars.
    #[test]
    fn address_uses_only_bech32_charset() {
        let s = encode(&ZERO_LEAF);
        for (i, c) in s.chars().enumerate() {
            if i < 5 {
                continue; // skip "qash1"
            }
            assert!(
                CHARSET.contains(&(c as u8)),
                "character {:?} at position {} is not in Bech32 charset",
                c,
                i
            );
        }
    }
}
