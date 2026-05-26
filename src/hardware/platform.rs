//! Platform-specific hardware abstraction — Domain B Phase 2 placeholder.
//!
//! Provides a uniform view of the underlying hardware platform for Domain B
//! operational code. No platform values may flow into Domain A state fields.

use super::capabilities::HardwareCapabilities;

/// Identifies the hardware platform type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformKind {
    /// Standard x86-64 server or workstation.
    X86_64Server,
    /// AArch64 server (Ampere, AWS Graviton, etc.).
    Aarch64Server,
    /// RISC-V embedded node.
    RiscV64Embedded,
    /// Unknown / not yet probed.
    Unknown,
}

/// Runtime platform descriptor (Domain B only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformDescriptor {
    pub kind: PlatformKind,
    pub capabilities: HardwareCapabilities,
}

impl PlatformDescriptor {
    /// Probe and describe the current platform. Returns a stub in Phase 1.
    pub fn probe() -> Self {
        Self {
            kind: PlatformKind::Unknown,
            capabilities: HardwareCapabilities::probe(),
        }
    }
}
