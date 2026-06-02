pub mod acceleration;
pub mod attestation_gate;
pub mod capabilities;
pub mod platform;
pub mod power_management;

// Feature-gated hardware attestation backends (scaffold; not claimed for v1.0).
#[cfg(feature = "arm-cca")]
pub mod arm_cca;
#[cfg(feature = "sev-snp")]
pub mod sev_snp;
#[cfg(feature = "tdx")]
pub mod tdx;
#[cfg(feature = "tpm2")]
pub mod tpm2;

pub use acceleration::{
    AccelerationBackend, AccelerationError, FieldOp, SoftwareAccelerationBackend,
};
pub use attestation_gate::{
    AttestationGate, AttestationGateError, AttestationQuote, LocalEvidenceAttestationGate,
    SoftwareHashMerkleAttestation, UnimplementedAttestationGate,
};
pub use capabilities::HardwareCapabilities;
pub use platform::{PlatformDescriptor, PlatformKind};
pub use power_management::{
    InMemoryPowerManager, PowerError, PowerManager, PowerState, UnimplementedPowerManager,
};

// Re-export hardware backend types when feature-gated.
#[cfg(feature = "arm-cca")]
pub use arm_cca::ArmCcaAttestationGate;
#[cfg(feature = "sev-snp")]
pub use sev_snp::SevSnpAttestationGate;
#[cfg(feature = "tdx")]
pub use tdx::TdxAttestationGate;
#[cfg(feature = "tpm2")]
pub use tpm2::Tpm2AttestationGate;
