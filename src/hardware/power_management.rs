//! Power management interface for Domain B hosted operation.
//!
//! Controls power states for validator nodes on embedded / OT platforms.
//! Power signals are Domain B operational material and must not influence
//! Domain A consensus transitions.

use std::sync::Mutex;

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

/// In-memory power manager for hosted tests and operators without platform hooks.
///
/// This manager records requested Domain B operational state only. It does not
/// suspend processes, control hardware, or feed power state into consensus.
#[derive(Debug)]
pub struct InMemoryPowerManager {
    state: Mutex<PowerState>,
}

impl Default for InMemoryPowerManager {
    fn default() -> Self {
        Self::new(PowerState::Active)
    }
}

impl InMemoryPowerManager {
    pub const fn new(initial_state: PowerState) -> Self {
        Self {
            state: Mutex::new(initial_state),
        }
    }
}

impl PowerManager for InMemoryPowerManager {
    fn request_state(&self, state: PowerState) -> Result<(), PowerError> {
        let mut current = self
            .state
            .lock()
            .map_err(|_| PowerError::Failed("power state lock poisoned".to_owned()))?;
        *current = state;
        Ok(())
    }

    fn current_state(&self) -> PowerState {
        self.state
            .lock()
            .map(|state| *state)
            .unwrap_or(PowerState::Active)
    }
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

#[cfg(test)]
mod tests {
    use super::{
        InMemoryPowerManager, PowerError, PowerManager, PowerState, UnimplementedPowerManager,
    };

    #[test]
    fn in_memory_manager_defaults_to_active() {
        let manager = InMemoryPowerManager::default();

        assert_eq!(manager.current_state(), PowerState::Active);
    }

    #[test]
    fn in_memory_manager_records_requested_states() {
        let manager = InMemoryPowerManager::default();

        manager
            .request_state(PowerState::Idle)
            .expect("idle transition is recorded");
        assert_eq!(manager.current_state(), PowerState::Idle);

        manager
            .request_state(PowerState::GracefulShutdown)
            .expect("shutdown transition is recorded");
        assert_eq!(manager.current_state(), PowerState::GracefulShutdown);
    }

    #[test]
    fn unimplemented_manager_fails_closed() {
        let manager = UnimplementedPowerManager;

        assert_eq!(
            manager.request_state(PowerState::Idle),
            Err(PowerError::NotSupported)
        );
        assert_eq!(manager.current_state(), PowerState::Active);
    }
}
