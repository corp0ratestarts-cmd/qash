pub mod acceleration;
pub mod attestation_gate;
pub mod capabilities;
pub mod platform;
pub mod power_management;

pub use acceleration::{
    AccelerationBackend, AccelerationError, FieldOp, SoftwareAccelerationBackend,
};
pub use attestation_gate::{
    AttestationGate, AttestationGateError, AttestationQuote, UnimplementedAttestationGate,
};
pub use capabilities::HardwareCapabilities;
pub use platform::{PlatformDescriptor, PlatformKind};
pub use power_management::{PowerError, PowerManager, PowerState, UnimplementedPowerManager};
