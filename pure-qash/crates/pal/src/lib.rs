//! Pure QASH Platform Abstraction Layer (Domain B).
//!
//! Domain B rules:
//!   - unsafe permitted under audit
//!   - Domain B values must NEVER flow into Domain A computations
//!   - zero-persistence production profile: no raw graph material in any durable store

pub mod admission;
pub mod wal;
