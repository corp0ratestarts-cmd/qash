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

    fn platform_measurement(&self) -> [u8; 32] {
        let mut hasher = Sha3_256::new();
        hasher.update(SOFTWARE_BACKEND_MEASUREMENT_DOMAIN);
        hasher.update(b"software-fallback-v1");
        hasher.finalize().into()
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
    use super::{AccelerationBackend, AccelerationError, SoftwareAccelerationBackend};

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
}
