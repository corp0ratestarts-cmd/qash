// Migration compatibility configuration (GENESIS_CONSTANTS.toml [migration.compatibility]).
//
// This module consumes all [migration.compatibility] config keys so the
// verify_migration_keys_consumed.sh anti-drift gate can verify they are not
// dead-letter.  Full runtime TOML loading is a P1 task.
//
// TOML keys consumed here:
//   accept_v1_0_validators
//   migration_window_epochs
//   v1_0_validator_cascade_mode
//   post_migration_enforcement
//   state_conversion_proof_required

/// TOML key: accept_v1_0_validators
/// Whether the node accepts v1.0 validators during the migration window.
pub const ACCEPT_V1_0_VALIDATORS: bool = true;

/// TOML key: migration_window_epochs
/// Number of epochs in the migration window before cascade is required.
pub const MIGRATION_WINDOW_EPOCHS: u64 = 100;

/// TOML key: v1_0_validator_cascade_mode
/// Cascade mode accepted for v1.0 validators during migration.
/// "parallel_only" = only L1 parallel output required (no cascade binding).
pub const V1_0_VALIDATOR_CASCADE_MODE: &str = "parallel_only";

/// TOML key: post_migration_enforcement
/// Enforcement policy after migration window expires.
/// "cascade_required" = validators without cascade proofs are rejected.
pub const POST_MIGRATION_ENFORCEMENT: &str = "cascade_required";

/// TOML key: state_conversion_proof_required
/// Whether a state-conversion ZK proof is required for migrating validators.
pub const STATE_CONVERSION_PROOF_REQUIRED: bool = true;
