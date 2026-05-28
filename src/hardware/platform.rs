//! Platform-specific hardware abstraction — Domain B reporting only.
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
    /// LoongArch host.
    LoongArch64,
    /// ARM 32-bit host.
    Arm,
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
    /// Probe and describe the current platform for operator diagnostics.
    pub fn probe() -> Self {
        Self {
            kind: PlatformKind::current(),
            capabilities: HardwareCapabilities::probe(),
        }
    }
}

impl PlatformKind {
    pub fn current() -> Self {
        Self::from_arch(std::env::consts::ARCH)
    }

    fn from_arch(arch: &str) -> Self {
        match arch {
            "x86_64" => Self::X86_64Server,
            "aarch64" => Self::Aarch64Server,
            "riscv64" => Self::RiscV64Embedded,
            "loongarch64" => Self::LoongArch64,
            "arm" | "armv7" => Self::Arm,
            _ => Self::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PlatformDescriptor, PlatformKind};

    #[test]
    fn known_architectures_map_to_descriptors() {
        assert_eq!(
            PlatformKind::from_arch("x86_64"),
            PlatformKind::X86_64Server
        );
        assert_eq!(
            PlatformKind::from_arch("aarch64"),
            PlatformKind::Aarch64Server
        );
        assert_eq!(
            PlatformKind::from_arch("riscv64"),
            PlatformKind::RiscV64Embedded
        );
        assert_eq!(PlatformKind::from_arch("unknown"), PlatformKind::Unknown);
    }

    #[test]
    fn platform_probe_reports_current_kind() {
        let descriptor = PlatformDescriptor::probe();
        assert_eq!(descriptor.kind, PlatformKind::current());
    }
}
