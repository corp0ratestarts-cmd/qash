//! Power management interface — Domain B Phase 2 placeholder.
//!
//! Controls power states for validator nodes on embedded / OT platforms.
//! Power signals are Domain B operational material and must not influence
//! Domain A consensus transitions.

/// Power management error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PowerError {
    /// Power management not supported on this platform.
    NotSupported,
    /// Failed to apply the requested power state.
    Failed(String),
}

/// Requested power state for a validator node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerState {
    /// Normal operational state.
    Active,
    /// Low-power idle (no consensus participation).
    Idle,
    /// Controlled shutdown after completing the current epoch.
    GracefulShutdown,
}

/// Domain B power management trait.
pub trait PowerManager {
    fn request_state(&self, state: PowerState) -> Result<(), PowerError>;
    fn current_state(&self) -> PowerState;
}

/// Stub that reports `Active` and ignores state transitions. Replace in Phase 2.
pub struct UnimplementedPowerManager;

impl PowerManager for UnimplementedPowerManager {
    fn request_state(&self, _state: PowerState) -> Result<(), PowerError> {
        Err(PowerError::NotSupported)
    }

    fn current_state(&self) -> PowerState {
        PowerState::Active
    }
}
