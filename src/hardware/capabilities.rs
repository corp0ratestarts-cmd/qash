//! Hardware capability detection — Domain B reporting only.
//!
//! Detects platform features available for Domain B acceleration and
//! attestation. Results are never fed into Domain A computations.

use std::path::Path;

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
    /// Probe local platform capabilities for operator diagnostics.
    pub fn probe() -> Self {
        Self {
            tpm2_available: tpm2_device_present(),
            confidential_compute: confidential_compute_hint(),
            hash_acceleration: hash_acceleration_hint(),
            aes_acceleration: aes_acceleration_hint(),
        }
    }

    /// Returns `true` if any hardware attestation path is available.
    pub fn has_attestation(&self) -> bool {
        self.tpm2_available || self.confidential_compute
    }
}

fn tpm2_device_present() -> bool {
    Path::new("/dev/tpmrm0").exists() || Path::new("/dev/tpm0").exists()
}

fn confidential_compute_hint() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        Path::new("/sys/firmware/sev").exists()
            || Path::new("/sys/firmware/tdx").exists()
            || Path::new("/dev/sev-guest").exists()
            || Path::new("/dev/tdx-guest").exists()
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        false
    }
}

fn hash_acceleration_hint() -> bool {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        std::is_x86_feature_detected!("sha")
    }
    #[cfg(target_arch = "aarch64")]
    {
        std::arch::is_aarch64_feature_detected!("sha2")
            || std::arch::is_aarch64_feature_detected!("sha3")
    }
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")))]
    {
        false
    }
}

fn aes_acceleration_hint() -> bool {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        std::is_x86_feature_detected!("aes")
    }
    #[cfg(target_arch = "aarch64")]
    {
        std::arch::is_aarch64_feature_detected!("aes")
    }
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::HardwareCapabilities;

    #[test]
    fn probe_is_reporting_only_and_non_panicking() {
        let caps = HardwareCapabilities::probe();
        assert_eq!(
            caps.has_attestation(),
            caps.tpm2_available || caps.confidential_compute
        );
    }
}
