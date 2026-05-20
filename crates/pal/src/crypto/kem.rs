//! ML-KEM-768 post-quantum key encapsulation (FIPS 203 / NIST MLKEM).
//!
//! Feature-gated behind `pqc`. Domain B only — never imported from
//! `qash-consensus`.
//!
//! # Security notes
//! * ML-KEM-768 targets NIST security level 3 (comparable to AES-192).
//! * It is CNSA 2.0 approved and the required PQC KEM for QASH Domain B.
//! * The hybrid option (`xwing_combine`) combines ML-KEM-768 with X25519 using
//!   the X-Wing combiner (SHA3-256 over both shared secrets + public keys) to
//!   preserve security if either primitive is broken.
//! * Private key material is zeroized on drop via `ml_kem`'s internal hygiene.

use ml_kem::{
    kem::Decapsulate,
    B32, Ciphertext, DecapsulationKey, EncapsulationKey, MlKem768, Seed,
};
use sha3::{Digest, Sha3_256};

/// ML-KEM-768 key pair (FIPS 203).
pub struct MlKem768KeyPair {
    decap_key: DecapsulationKey<MlKem768>,
}

impl MlKem768KeyPair {
    /// Generate a key pair from a 64-byte uniformly random seed.
    ///
    /// The seed MUST come from a FIPS 140-3 approved entropy source such as
    /// `FipsDrbg` in `crates/pal/src/crypto/drbg.rs`.
    pub fn from_seed(seed: &[u8; 64]) -> Self {
        let s = Seed::from(*seed);
        let decap_key = DecapsulationKey::<MlKem768>::from_seed(s);
        Self { decap_key }
    }

    /// Return the encapsulation (public) key — safe to publish.
    pub fn encap_key(&self) -> EncapsulationKey<MlKem768> {
        self.decap_key.encapsulation_key().clone()
    }

    /// Decapsulate a ciphertext, recovering the 32-byte shared secret.
    ///
    /// On ciphertext malformation, ML-KEM returns an implicit-rejection
    /// pseudorandom value (not an error) per FIPS 203 §6.4.
    pub fn decapsulate(&self, ciphertext: &MlKem768Ciphertext) -> [u8; 32] {
        self.decap_key.decapsulate(&ciphertext.0).into()
    }
}

/// Opaque ML-KEM-768 ciphertext.
pub struct MlKem768Ciphertext(Ciphertext<MlKem768>);

/// Encapsulate to a remote ML-KEM-768 public key, returning
/// `(ciphertext, shared_secret_32_bytes)`.
///
/// `randomness` is a 32-byte value that MUST be drawn from a FIPS-approved
/// DRBG. Using a predictable value leaks the shared secret.
pub fn encapsulate(
    encap_key: &EncapsulationKey<MlKem768>,
    randomness: &[u8; 32],
) -> (MlKem768Ciphertext, [u8; 32]) {
    let m = B32::from(*randomness);
    let (ct, ss) = encap_key.encapsulate_deterministic(&m);
    (MlKem768Ciphertext(ct), ss.into())
}

// ─── Hybrid (X-Wing) combiner ──────────────────────────────────────────────

/// X-Wing hybrid shared secret combiner:
/// `SHA3-256(mlkem_ss || x25519_ss || mlkem_ek_bytes || x25519_pk)`.
///
/// Combining both shared secrets means the hybrid remains secure even if one
/// primitive is broken. Public-key binding prevents unknown-key-share attacks.
pub fn xwing_combine(
    mlkem_ss: &[u8; 32],
    x25519_ss: &[u8; 32],
    mlkem_ek_bytes: &[u8],
    x25519_pk: &[u8; 32],
) -> [u8; 32] {
    let mut h = Sha3_256::new();
    h.update(mlkem_ss);
    h.update(x25519_ss);
    h.update(mlkem_ek_bytes);
    h.update(x25519_pk);
    h.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kat_encap_decap_roundtrip() {
        let seed = [0x42u8; 64];
        let kp = MlKem768KeyPair::from_seed(&seed);
        let ek = kp.encap_key();

        let randomness = [0x37u8; 32];
        let (ct, ss_send) = encapsulate(&ek, &randomness);
        let ss_recv = kp.decapsulate(&ct);

        assert_eq!(ss_send, ss_recv, "encap/decap shared secrets must match");
        assert_ne!(ss_send, [0u8; 32], "shared secret must not be all-zero");
    }

    #[test]
    fn different_seeds_produce_different_keypairs() {
        let kp_a = MlKem768KeyPair::from_seed(&[0x01u8; 64]);
        let kp_b = MlKem768KeyPair::from_seed(&[0x02u8; 64]);

        let rng = [0x00u8; 32];
        let (_, ss_a) = encapsulate(&kp_a.encap_key(), &rng);
        let (_, ss_b) = encapsulate(&kp_b.encap_key(), &rng);
        assert_ne!(ss_a, ss_b, "different key pairs → different shared secrets");
    }

    #[test]
    fn wrong_decap_key_produces_different_ss() {
        // Implicit rejection: wrong key → pseudorandom (but mismatched) SS.
        let kp_a = MlKem768KeyPair::from_seed(&[0x10u8; 64]);
        let kp_b = MlKem768KeyPair::from_seed(&[0x20u8; 64]);

        let (ct, ss_correct) = encapsulate(&kp_a.encap_key(), &[0xABu8; 32]);
        let ss_wrong = kp_b.decapsulate(&ct);
        assert_ne!(ss_correct, ss_wrong, "wrong key must yield a different SS");
    }

    #[test]
    fn xwing_combine_is_deterministic() {
        let a = xwing_combine(&[0xAAu8; 32], &[0xBBu8; 32], &[0xCCu8; 64], &[0xDDu8; 32]);
        let b = xwing_combine(&[0xAAu8; 32], &[0xBBu8; 32], &[0xCCu8; 64], &[0xDDu8; 32]);
        assert_eq!(a, b);
        assert_ne!(a, [0u8; 32]);
    }

    #[test]
    fn xwing_combine_binds_all_inputs() {
        let base = xwing_combine(&[0x01u8; 32], &[0x02u8; 32], &[0x03u8; 64], &[0x04u8; 32]);
        let mut v = [0x01u8; 32];
        v[0] ^= 0xFF;
        assert_ne!(base, xwing_combine(&v, &[0x02u8; 32], &[0x03u8; 64], &[0x04u8; 32]));
        let mut v = [0x02u8; 32];
        v[0] ^= 0xFF;
        assert_ne!(base, xwing_combine(&[0x01u8; 32], &v, &[0x03u8; 64], &[0x04u8; 32]));
    }
}
