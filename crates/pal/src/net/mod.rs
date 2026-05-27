//! Domain B network transport implementations.
//!
//! Provides a TCP-backed `CommitmentTransport` and test utilities for
//! simulating network faults (drops, reorders, delays). All code here
//! is Domain B and may use `std`. Nothing in this module crosses the
//! Domain A/B boundary — only encoded `CommitmentFrame` bytes are transmitted.

pub mod tcp_transport;

#[cfg(any(test, feature = "std"))]
pub mod faulty_transport;
