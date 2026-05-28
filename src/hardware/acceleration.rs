//! Domain B acceleration backend interfaces.
//!
//! Hardware acceleration is optional operator infrastructure. The software
//! backend below is deterministic and exists so callers can exercise the
//! acceleration boundary without depending on platform devices.

use sha3::{Digest, Sha3_256};

const SOFTWARE_BACKEND_MEASUREMENT_DOMAIN: &[u8] = b"QASH-DOMAIN-B-SOFTWARE-ACCELERATION\0";

pub trait AccelerationBackend {
    fn accelerate_hash_cascade(
        &self,
        input: &[u8],
        domain: &[u8],
        output: &mut [u8; 32],
    ) -> Result<(), AccelerationError>;

    fn accelerate_field_ops(
        &self,
        _a: &[u8; 32],
        _b: &[u8; 32],
        _prime: &[u8; 32],
        _op: FieldOp,
    ) -> Result<[u8; 32], AccelerationError> {
        Err(AccelerationError::NotImplemented)
    }

    fn platform_measurement(&self) -> [u8; 32];
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SoftwareAccelerationBackend;

impl AccelerationBackend for SoftwareAccelerationBackend {
    fn accelerate_hash_cascade(
        &self,
        input: &[u8],
        domain: &[u8],
        output: &mut [u8; 32],
    ) -> Result<(), AccelerationError> {
        if domain.is_empty() {
            return Err(AccelerationError::InvalidInput);
        }
        let cascade = crate::crypto::cascade::h_cascade_keyed(domain, input);
        output.copy_from_slice(&cascade[..32]);
        Ok(())
    }

    /// Modular arithmetic on 256-bit little-endian values.
    ///
    /// `a`, `b`, and `prime` are 32-byte little-endian unsigned integers.
    /// All operations return a result reduced modulo `prime`.
    fn accelerate_field_ops(
        &self,
        a: &[u8; 32],
        b: &[u8; 32],
        prime: &[u8; 32],
        op: FieldOp,
    ) -> Result<[u8; 32], AccelerationError> {
        let a_val = le32_to_u256(a);
        let b_val = le32_to_u256(b);
        let p_val = le32_to_u256(prime);

        if p_val == U256::ZERO {
            return Err(AccelerationError::InvalidInput);
        }

        let result = match op {
            FieldOp::Add => u256_mod_add(a_val, b_val, p_val),
            FieldOp::Mul => u256_mod_mul(a_val, b_val, p_val),
            FieldOp::Mod => u256_mod(a_val, p_val),
        };

        Ok(u256_to_le32(result))
    }

    fn platform_measurement(&self) -> [u8; 32] {
        let mut hasher = Sha3_256::new();
        hasher.update(SOFTWARE_BACKEND_MEASUREMENT_DOMAIN);
        hasher.update(b"software-fallback-v1");
        hasher.finalize().into()
    }
}

// ---------------------------------------------------------------------------
// Minimal 256-bit integer arithmetic (little-endian u128 pair)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
struct U256 {
    lo: u128,
    hi: u128,
}

impl U256 {
    const ZERO: Self = Self { lo: 0, hi: 0 };
}

fn le32_to_u256(b: &[u8; 32]) -> U256 {
    let mut lo_bytes = [0u8; 16];
    let mut hi_bytes = [0u8; 16];
    lo_bytes.copy_from_slice(&b[..16]);
    hi_bytes.copy_from_slice(&b[16..]);
    U256 {
        lo: u128::from_le_bytes(lo_bytes),
        hi: u128::from_le_bytes(hi_bytes),
    }
}

fn u256_to_le32(v: U256) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[..16].copy_from_slice(&v.lo.to_le_bytes());
    out[16..].copy_from_slice(&v.hi.to_le_bytes());
    out
}

fn u256_add_wrapping(a: U256, b: U256) -> (U256, bool) {
    let (lo, carry_lo) = a.lo.overflowing_add(b.lo);
    let (hi_tmp, carry_hi1) = a.hi.overflowing_add(b.hi);
    let (hi, carry_hi2) = hi_tmp.overflowing_add(carry_lo as u128);
    (U256 { lo, hi }, carry_hi1 || carry_hi2)
}

fn u256_lt(a: U256, b: U256) -> bool {
    if a.hi != b.hi {
        a.hi < b.hi
    } else {
        a.lo < b.lo
    }
}

fn u256_sub(a: U256, b: U256) -> U256 {
    let (lo, borrow) = a.lo.overflowing_sub(b.lo);
    let hi = a.hi.wrapping_sub(b.hi).wrapping_sub(borrow as u128);
    U256 { lo, hi }
}

fn u256_mod_add(a: U256, b: U256, p: U256) -> U256 {
    let a = if u256_lt(a, p) { a } else { u256_mod(a, p) };
    let b = if u256_lt(b, p) { b } else { u256_mod(b, p) };
    let (sum, overflow) = u256_add_wrapping(a, b);
    if overflow || !u256_lt(sum, p) {
        u256_sub(sum, p)
    } else {
        sum
    }
}

/// Modular multiplication via binary double-and-add.
///
/// All intermediate values stay ≤ p-1 because every addition is immediately
/// reduced mod p. No 512-bit intermediates, no overflow risk.
fn u256_mod_mul(a: U256, b: U256, p: U256) -> U256 {
    let mut result = U256::ZERO;
    let mut base = if u256_lt(b, p) { b } else { u256_mod(b, p) };
    let mut a_bits = if u256_lt(a, p) { a } else { u256_mod(a, p) };

    for _ in 0..256_u32 {
        if a_bits.lo & 1 == 1 {
            result = u256_mod_add(result, base, p);
        }
        base = u256_mod_add(base, base, p);
        let carry_bit = (a_bits.hi & 1) << 127;
        a_bits.lo = (a_bits.lo >> 1) | carry_bit;
        a_bits.hi >>= 1;
    }

    result
}

fn u256_mod(a: U256, p: U256) -> U256 {
    if u256_lt(a, p) {
        return a;
    }
    let a_bits = u256_bit_len(a);
    let p_bits = u256_bit_len(p);
    let mut rem = a;
    let mut shift = a_bits.saturating_sub(p_bits);
    loop {
        let shifted_p = u256_shl(p, shift);
        if !u256_lt(rem, shifted_p) {
            rem = u256_sub(rem, shifted_p);
        }
        if shift == 0 {
            break;
        }
        shift -= 1;
    }
    rem
}

fn u256_bit_len(v: U256) -> usize {
    if v.hi != 0 {
        let sig = (u128::BITS - v.hi.leading_zeros()) as usize;
        128_usize.saturating_add(sig)
    } else if v.lo != 0 {
        (u128::BITS - v.lo.leading_zeros()) as usize
    } else {
        0
    }
}

fn u256_shl(v: U256, shift: usize) -> U256 {
    if shift == 0 {
        return v;
    }
    if shift >= 256 {
        return U256::ZERO;
    }
    if shift >= 128 {
        let s = shift.saturating_sub(128);
        U256 {
            lo: 0,
            hi: v.lo << s,
        }
    } else {
        let lo = v.lo << shift;
        let carry = v.lo >> (128_usize.saturating_sub(shift));
        let hi = (v.hi << shift) | carry;
        U256 { lo, hi }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccelerationError {
    NotImplemented,
    InvalidInput,
    VerificationFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldOp {
    Add,
    Mul,
    Mod,
}

#[cfg(test)]
mod tests {
    use super::{AccelerationBackend, AccelerationError, FieldOp, SoftwareAccelerationBackend};

    fn le32(v: u64) -> [u8; 32] {
        let mut out = [0u8; 32];
        out[..8].copy_from_slice(&v.to_le_bytes());
        out
    }

    #[test]
    fn software_backend_hash_cascade_is_deterministic() {
        let backend = SoftwareAccelerationBackend;
        let mut first = [0u8; 32];
        let mut second = [0u8; 32];

        backend
            .accelerate_hash_cascade(b"payload", b"domain", &mut first)
            .expect("valid input");
        backend
            .accelerate_hash_cascade(b"payload", b"domain", &mut second)
            .expect("valid input");

        assert_eq!(first, second);
        assert_ne!(first, [0u8; 32]);
    }

    #[test]
    fn software_backend_binds_domain_and_input() {
        let backend = SoftwareAccelerationBackend;
        let mut baseline = [0u8; 32];
        let mut changed_domain = [0u8; 32];
        let mut changed_input = [0u8; 32];

        backend
            .accelerate_hash_cascade(b"payload", b"domain-a", &mut baseline)
            .unwrap();
        backend
            .accelerate_hash_cascade(b"payload", b"domain-b", &mut changed_domain)
            .unwrap();
        backend
            .accelerate_hash_cascade(b"payload-2", b"domain-a", &mut changed_input)
            .unwrap();

        assert_ne!(baseline, changed_domain);
        assert_ne!(baseline, changed_input);
    }

    #[test]
    fn software_backend_rejects_empty_domain() {
        let backend = SoftwareAccelerationBackend;
        let mut output = [0u8; 32];

        assert_eq!(
            backend.accelerate_hash_cascade(b"payload", b"", &mut output),
            Err(AccelerationError::InvalidInput)
        );
    }

    #[test]
    fn software_backend_measurement_is_stable() {
        let backend = SoftwareAccelerationBackend;

        assert_eq!(
            backend.platform_measurement(),
            backend.platform_measurement()
        );
        assert_ne!(backend.platform_measurement(), [0u8; 32]);
    }

    #[test]
    fn field_add_basic() {
        let b = SoftwareAccelerationBackend;
        // 3 + 4 mod 7 = 0
        let r = b
            .accelerate_field_ops(&le32(3), &le32(4), &le32(7), FieldOp::Add)
            .unwrap();
        assert_eq!(&r[..8], &0u64.to_le_bytes());
    }

    #[test]
    fn field_add_with_wrap() {
        let b = SoftwareAccelerationBackend;
        // 5 + 5 mod 7 = 3
        let r = b
            .accelerate_field_ops(&le32(5), &le32(5), &le32(7), FieldOp::Add)
            .unwrap();
        assert_eq!(&r[..8], &3u64.to_le_bytes());
    }

    #[test]
    fn field_mul_basic() {
        let b = SoftwareAccelerationBackend;
        // 3 * 4 mod 7 = 5
        let r = b
            .accelerate_field_ops(&le32(3), &le32(4), &le32(7), FieldOp::Mul)
            .unwrap();
        assert_eq!(&r[..8], &5u64.to_le_bytes());
    }

    #[test]
    fn field_mod_basic() {
        let b = SoftwareAccelerationBackend;
        // 10 mod 7 = 3
        let r = b
            .accelerate_field_ops(&le32(10), &le32(0), &le32(7), FieldOp::Mod)
            .unwrap();
        assert_eq!(&r[..8], &3u64.to_le_bytes());
    }

    #[test]
    fn field_zero_prime_rejected() {
        let b = SoftwareAccelerationBackend;
        assert_eq!(
            b.accelerate_field_ops(&le32(1), &le32(1), &le32(0), FieldOp::Add),
            Err(AccelerationError::InvalidInput)
        );
    }
}
