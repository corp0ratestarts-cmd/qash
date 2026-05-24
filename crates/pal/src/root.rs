pub mod admission;
pub mod attestation;
pub mod commitment_backpressure;
pub mod commitment_inbox;
pub mod commitment_transport;
pub mod ingress;
pub mod receipt;
pub mod recovery_wal;
pub mod zero_wal;

// The legacy hosted PAL contains replay-scaffold machinery, including raw
// fixture handling used by tests and golden-vector generation. Production
// zero-persistence builds exclude that scaffold unless `replay-scaffold` is
// explicitly enabled.
#[cfg(not(all(feature = "zero-persistence", not(feature = "replay-scaffold"))))]
include!("lib.rs");
