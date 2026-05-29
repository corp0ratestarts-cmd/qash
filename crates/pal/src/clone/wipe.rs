// Emergency wipe signal for clone protocol.
//
// GENESIS_CONSTANTS.toml [clone_protocol]: emergency_wipe_signal = true
//
// A validator may broadcast a signed emergency wipe command that instructs
// all relay nodes and peers to purge their clone buffers for a specific
// session (identified by a 32-byte session_id).
//
// Wire format (wipe signal):
//   WIPE_MAGIC(8) || version(1) || session_id(32) || epoch_le8 || sig_pk(32) || sig(64)
//   = 145 bytes total
//
// Signature: SHA3-256("QASH/clone/wipe/v1\0" || session_id || epoch_le8),
// signed with the issuer's Ed25519-equivalent 64-byte signing key.
//
// Since Domain A has no post-quantum signature backend wired yet, this module
// uses a keyed SHA3-256 MAC as a stand-in signature scheme.  The wire format
// is stable; callers substitute the real PQC signature scheme when available.
//
// Domain B only.

use sha3::{Digest, Sha3_256};

/// Wire magic for emergency wipe signals.
pub const WIPE_MAGIC: &[u8; 8] = b"QASHWPE\0";

/// Current wipe signal wire version.
pub const WIPE_VERSION: u8 = 0x01;

/// Total byte length of a serialised wipe signal.
pub const WIPE_SIGNAL_BYTES: usize = 8 + 1 + 32 + 8 + 32 + 64;

const WIPE_SIGN_DOMAIN: &[u8] = b"QASH/clone/wipe/v1\0";
const WIPE_SIG_DOMAIN: &[u8] = b"QASH/clone/wipe/sig/v1\0";

/// A parsed emergency wipe signal.
#[derive(Clone, Debug)]
pub struct WipeSignal {
    /// 32-byte session identifier to wipe.
    pub session_id: [u8; 32],
    /// Epoch at which the wipe was issued.
    pub epoch: u64,
    /// Issuer's 32-byte public key.
    pub issuer_pk: [u8; 32],
    /// 64-byte keyed-hash "signature" over the wipe body.
    pub signature: [u8; 64],
}

/// Error type for wipe signal operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WipeError {
    InvalidLength,
    BadMagic,
    UnknownVersion,
    SignatureInvalid,
}

impl WipeSignal {
    /// Build and sign a wipe signal.
    ///
    /// `signing_key` is the issuer's 32-byte secret key (keyed-SHA3-256 scheme).
    pub fn sign(
        session_id: [u8; 32],
        epoch: u64,
        issuer_pk: [u8; 32],
        signing_key: &[u8; 32],
    ) -> Self {
        let sig = compute_signature(&session_id, epoch, &issuer_pk, signing_key);
        Self { session_id, epoch, issuer_pk, signature: sig }
    }

    /// Serialise to wire format (WIPE_SIGNAL_BYTES bytes).
    pub fn to_bytes(&self) -> [u8; WIPE_SIGNAL_BYTES] {
        let mut out = [0u8; WIPE_SIGNAL_BYTES];
        let mut pos = 0;
        out[pos..pos + 8].copy_from_slice(WIPE_MAGIC);
        pos += 8;
        out[pos] = WIPE_VERSION;
        pos += 1;
        out[pos..pos + 32].copy_from_slice(&self.session_id);
        pos += 32;
        out[pos..pos + 8].copy_from_slice(&self.epoch.to_le_bytes());
        pos += 8;
        out[pos..pos + 32].copy_from_slice(&self.issuer_pk);
        pos += 32;
        out[pos..pos + 64].copy_from_slice(&self.signature);
        out
    }

    /// Parse from wire bytes without verifying the signature.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, WipeError> {
        if bytes.len() != WIPE_SIGNAL_BYTES {
            return Err(WipeError::InvalidLength);
        }
        if &bytes[..8] != WIPE_MAGIC {
            return Err(WipeError::BadMagic);
        }
        if bytes[8] != WIPE_VERSION {
            return Err(WipeError::UnknownVersion);
        }
        let session_id: [u8; 32] = bytes[9..41].try_into().unwrap();
        let epoch = u64::from_le_bytes(bytes[41..49].try_into().unwrap());
        let issuer_pk: [u8; 32] = bytes[49..81].try_into().unwrap();
        let signature: [u8; 64] = bytes[81..145].try_into().unwrap();
        Ok(Self { session_id, epoch, issuer_pk, signature })
    }

    /// Verify the signature against `issuer_pk` using `verifier_key`.
    ///
    /// In the keyed-SHA3-256 scheme used here, `verifier_key` is the same
    /// as the signing key (symmetric MAC). Substitute a PQC verify call when
    /// the Dilithium5 PAL backend is available.
    pub fn verify(&self, verifier_key: &[u8; 32]) -> Result<(), WipeError> {
        let expected = compute_signature(&self.session_id, self.epoch, &self.issuer_pk, verifier_key);
        if expected != self.signature {
            return Err(WipeError::SignatureInvalid);
        }
        Ok(())
    }
}

fn compute_signature(
    session_id: &[u8; 32],
    epoch: u64,
    issuer_pk: &[u8; 32],
    key: &[u8; 32],
) -> [u8; 64] {
    // Body hash: SHA3-256(WIPE_SIGN_DOMAIN || session_id || epoch_le8)
    let mut body_h = Sha3_256::new();
    body_h.update(WIPE_SIGN_DOMAIN);
    body_h.update(session_id);
    body_h.update(epoch.to_le_bytes());
    let body_hash: [u8; 32] = body_h.finalize().into();

    // Signature: SHA3-256(WIPE_SIG_DOMAIN || key || issuer_pk || body_hash)
    // Expanded to 64 bytes via two successive hashes.
    let mut sig_h = Sha3_256::new();
    sig_h.update(WIPE_SIG_DOMAIN);
    sig_h.update(key);
    sig_h.update(issuer_pk);
    sig_h.update(&body_hash);
    let lo: [u8; 32] = sig_h.finalize().into();

    let mut sig_h2 = Sha3_256::new();
    sig_h2.update(WIPE_SIG_DOMAIN);
    sig_h2.update(b"ext\0");
    sig_h2.update(&lo);
    let hi: [u8; 32] = sig_h2.finalize().into();

    let mut out = [0u8; 64];
    out[..32].copy_from_slice(&lo);
    out[32..].copy_from_slice(&hi);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_signal() -> (WipeSignal, [u8; 32]) {
        let sk = [0x42u8; 32];
        let pk = [0x43u8; 32];
        let session = [0x01u8; 32];
        let sig = WipeSignal::sign(session, 100, pk, &sk);
        (sig, sk)
    }

    #[test]
    fn wire_length_is_correct() {
        let (sig, _) = test_signal();
        assert_eq!(sig.to_bytes().len(), WIPE_SIGNAL_BYTES);
        assert_eq!(WIPE_SIGNAL_BYTES, 145);
    }

    #[test]
    fn roundtrip_bytes() {
        let (sig, _) = test_signal();
        let bytes = sig.to_bytes();
        let parsed = WipeSignal::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.session_id, sig.session_id);
        assert_eq!(parsed.epoch, sig.epoch);
        assert_eq!(parsed.issuer_pk, sig.issuer_pk);
        assert_eq!(parsed.signature, sig.signature);
    }

    #[test]
    fn verify_succeeds_with_correct_key() {
        let (sig, sk) = test_signal();
        sig.verify(&sk).unwrap();
    }

    #[test]
    fn verify_rejects_wrong_key() {
        let (sig, _) = test_signal();
        let wrong_key = [0xFFu8; 32];
        assert_eq!(sig.verify(&wrong_key), Err(WipeError::SignatureInvalid));
    }

    #[test]
    fn from_bytes_rejects_bad_magic() {
        let (sig, _) = test_signal();
        let mut bytes = sig.to_bytes();
        bytes[0] ^= 0xFF;
        assert_eq!(WipeSignal::from_bytes(&bytes).unwrap_err(), WipeError::BadMagic);
    }

    #[test]
    fn from_bytes_rejects_wrong_length() {
        assert_eq!(WipeSignal::from_bytes(&[0u8; 10]).unwrap_err(), WipeError::InvalidLength);
    }

    #[test]
    fn from_bytes_rejects_unknown_version() {
        let (sig, _) = test_signal();
        let mut bytes = sig.to_bytes();
        bytes[8] = 0xFF; // version byte
        assert_eq!(WipeSignal::from_bytes(&bytes).unwrap_err(), WipeError::UnknownVersion);
    }

    #[test]
    fn distinct_sessions_produce_distinct_signatures() {
        let sk = [1u8; 32];
        let pk = [2u8; 32];
        let a = WipeSignal::sign([0u8; 32], 1, pk, &sk);
        let b = WipeSignal::sign([1u8; 32], 1, pk, &sk);
        assert_ne!(a.signature, b.signature);
    }

    #[test]
    fn distinct_epochs_produce_distinct_signatures() {
        let sk = [1u8; 32];
        let pk = [2u8; 32];
        let session = [0u8; 32];
        let a = WipeSignal::sign(session, 1, pk, &sk);
        let b = WipeSignal::sign(session, 2, pk, &sk);
        assert_ne!(a.signature, b.signature);
    }
}
