//! Hardware capability detection — Domain B Phase 2 placeholder.
//!
//! Detects platform features available for Domain B acceleration and
//! attestation. Results are never fed into Domain A computations.

/// Detected hardware capabilities for this platform.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HardwareCapabilities {
    /// TPM 2.0 is present and accessible.
    pub tpm2_available: bool,
    /// Intel TDX or AMD SEV-SNP confidential computing is active.
    pub confidential_compute: bool,
    /// Hardware SHA-3 / SHA-256 acceleration is available.
    pub hash_acceleration: bool,
    /// AES-NI or equivalent symmetric acceleration is available.
    pub aes_acceleration: bool,
}

impl HardwareCapabilities {
    /// Probe platform capabilities. Currently returns a null-capability stub.
    /// Phase 2 will implement real CPUID / sysfs probing here.
    pub fn probe() -> Self {
        Self::default()
    }

    /// Returns `true` if any hardware attestation path is available.
    pub fn has_attestation(&self) -> bool {
        self.tpm2_available || self.confidential_compute
    }
}
