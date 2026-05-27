//! TLS configuration validation and log pseudonym helpers.
//!
//! `validate_tls_config()` — reject TLS 1.0/1.1 at PAL init time.
//! `log_pseudonym()` — SHA3-256 pseudonym for validator IDs in log lines.
//! Never log raw public keys, IP addresses, or validator identity bytes.

use sha3::{Digest, Sha3_256};

/// TLS version floor: only TLS 1.2 and above are accepted.
const TLS_VERSION_1_2: u16 = 0x0303;

/// Error returned when the TLS configuration is below the security floor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsConfigError {
    /// Negotiated or configured version is below TLS 1.2.
    VersionBelowFloor {
        negotiated: u16,
        minimum: u16,
    },
    /// No TLS configured; plaintext connections are not permitted.
    NoTls,
}

impl core::fmt::Display for TlsConfigError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TlsConfigError::VersionBelowFloor { negotiated, minimum } => {
                write!(
                    f,
                    "TLS version {:#06x} is below minimum {:#06x}",
                    negotiated, minimum
                )
            }
            TlsConfigError::NoTls => write!(f, "no TLS configured"),
        }
    }
}

/// Validate that a TLS version code meets the security floor (≥ TLS 1.2).
///
/// Call this at PAL initialisation time when wiring any network transport.
/// Returns `Ok(())` for TLS 1.2 (0x0303) and TLS 1.3 (0x0304).
/// Returns `Err(TlsConfigError::VersionBelowFloor)` for SSLv3, TLS 1.0, TLS 1.1.
/// Returns `Err(TlsConfigError::NoTls)` when `tls_version` is `None`.
pub fn validate_tls_config(tls_version: Option<u16>) -> Result<(), TlsConfigError> {
    match tls_version {
        None => Err(TlsConfigError::NoTls),
        Some(v) if v < TLS_VERSION_1_2 => Err(TlsConfigError::VersionBelowFloor {
            negotiated: v,
            minimum: TLS_VERSION_1_2,
        }),
        Some(_) => Ok(()),
    }
}

/// Derive a log-safe pseudonym for a validator public key or identity.
///
/// Returns the first 16 bytes of SHA3-256(prefix || pk_bytes) as a hex
/// string. The truncated hash is one-way: the original key cannot be
/// recovered from the log entry, and the full 32-byte hash is not emitted.
///
/// **Never log raw public keys, IP addresses, or validator IDs directly.**
/// Use this function to produce the identifier that appears in log lines.
///
/// `prefix` is a domain separator (e.g. `b"qash/log/validator/v1"`).
pub fn log_pseudonym(prefix: &[u8], pk_bytes: &[u8]) -> [u8; 16] {
    let mut h = Sha3_256::new();
    h.update(prefix);
    h.update(pk_bytes);
    let digest: [u8; 32] = h.finalize().into();
    let mut out = [0u8; 16];
    out.copy_from_slice(&digest[..16]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tls_12_is_accepted() {
        assert!(validate_tls_config(Some(0x0303)).is_ok());
    }

    #[test]
    fn tls_13_is_accepted() {
        assert!(validate_tls_config(Some(0x0304)).is_ok());
    }

    #[test]
    fn tls_11_is_rejected() {
        let err = validate_tls_config(Some(0x0302)).unwrap_err();
        assert_eq!(
            err,
            TlsConfigError::VersionBelowFloor {
                negotiated: 0x0302,
                minimum: 0x0303
            }
        );
    }

    #[test]
    fn tls_10_is_rejected() {
        assert!(matches!(
            validate_tls_config(Some(0x0301)),
            Err(TlsConfigError::VersionBelowFloor { .. })
        ));
    }

    #[test]
    fn sslv3_is_rejected() {
        assert!(matches!(
            validate_tls_config(Some(0x0300)),
            Err(TlsConfigError::VersionBelowFloor { .. })
        ));
    }

    #[test]
    fn no_tls_is_rejected() {
        assert_eq!(validate_tls_config(None), Err(TlsConfigError::NoTls));
    }

    #[test]
    fn log_pseudonym_is_deterministic() {
        let p = b"qash/log/validator/v1";
        let pk = [0xABu8; 48];
        let a = log_pseudonym(p, &pk);
        let b = log_pseudonym(p, &pk);
        assert_eq!(a, b);
    }

    #[test]
    fn log_pseudonym_changes_with_input() {
        let p = b"qash/log/validator/v1";
        let a = log_pseudonym(p, &[0xABu8; 48]);
        let b = log_pseudonym(p, &[0xCDu8; 48]);
        assert_ne!(a, b);
    }

    #[test]
    fn log_pseudonym_truncates_to_16_bytes() {
        let pseudo = log_pseudonym(b"prefix", b"pk_material");
        assert_eq!(pseudo.len(), 16);
    }

    #[test]
    fn log_pseudonym_is_not_raw_pk() {
        let pk = [0xABu8; 48];
        let pseudo = log_pseudonym(b"prefix", &pk);
        // 16-byte truncation cannot match raw pk (different length)
        assert_ne!(&pseudo, &pk[..16]);
    }
}
