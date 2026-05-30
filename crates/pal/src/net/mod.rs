//! Domain B network transport implementations.
//!
//! Provides a TCP-backed `CommitmentTransport` and test utilities for
//! simulating network faults (drops, reorders, delays). All code here
//! is Domain B and may use `std`. Nothing in this module crosses the
//! Domain A/B boundary — only encoded `CommitmentFrame` bytes are transmitted.
//!
//! # 4-B: PublicTranscript broadcast gate
//!
//! `publish_transcript_entry` is the ONLY authorised function for emitting
//! protocol state to a public-observable channel.  Raw `EpochState` MUST NOT
//! be serialised and transmitted directly — this function enforces the
//! Class I visibility boundary from `docs/spec/09_privacy_model.md §P3a`.

pub mod tcp_transport;

#[cfg(any(test, feature = "std"))]
pub mod faulty_transport;

use qash_consensus::public::PublicTranscript;

/// Error returned when the broadcast transport fails.
#[derive(Debug, PartialEq, Eq)]
pub enum NetError {
    TransportFull,
    IoError,
}

impl core::fmt::Display for NetError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TransportFull => write!(f, "network transport buffer full"),
            Self::IoError => write!(f, "network transport I/O error"),
        }
    }
}

/// Trait for any Domain B transport that can broadcast raw bytes.
///
/// Implementations must be infallible on normal operation; errors are
/// transient (buffer full, temporary I/O) not protocol violations.
pub trait NetTransport {
    fn broadcast(&self, bytes: &[u8]) -> Result<(), NetError>;
}

/// The ONLY function in Domain B authorised to write to a public channel.
///
/// Encodes `entry` via `PublicTranscript::encode_canonical` (105 bytes, no
/// PII, no graph edges) and broadcasts it via `transport`.
///
/// This function is intentionally NOT generic over `T`.  If you need to
/// broadcast raw `EpochState`, that is a privacy violation (§P3a).
pub fn publish_transcript_entry(
    transport: &impl NetTransport,
    entry: &PublicTranscript,
) -> Result<(), NetError> {
    let bytes = entry.encode_canonical();
    transport.broadcast(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use qash_consensus::public::PUBLIC_TRANSCRIPT_WIRE_LEN;

    struct CapturingTransport {
        captured: core::cell::Cell<Option<[u8; PUBLIC_TRANSCRIPT_WIRE_LEN]>>,
    }

    impl CapturingTransport {
        fn new() -> Self {
            Self {
                captured: core::cell::Cell::new(None),
            }
        }
        fn last_bytes(&self) -> Option<[u8; PUBLIC_TRANSCRIPT_WIRE_LEN]> {
            self.captured.get()
        }
    }

    impl NetTransport for CapturingTransport {
        fn broadcast(&self, bytes: &[u8]) -> Result<(), NetError> {
            let mut arr = [0u8; PUBLIC_TRANSCRIPT_WIRE_LEN];
            arr.copy_from_slice(bytes);
            self.captured.set(Some(arr));
            Ok(())
        }
    }

    #[test]
    fn publish_transcript_entry_broadcasts_canonical_encoding() {
        let transport = CapturingTransport::new();
        let pt = PublicTranscript {
            state_root: [0xAAu8; 32],
            receipt_root: [0xBBu8; 32],
            efb_root: [0xCCu8; 32],
            epoch: 42,
            halt_flag: false,
        };
        publish_transcript_entry(&transport, &pt).expect("broadcast ok");
        let received = transport.last_bytes().expect("bytes captured");
        let decoded = PublicTranscript::decode_canonical(&received).expect("canonical decode ok");
        assert_eq!(decoded, pt);
    }

    #[test]
    fn publish_transcript_entry_propagates_transport_error() {
        struct FailTransport;
        impl NetTransport for FailTransport {
            fn broadcast(&self, _: &[u8]) -> Result<(), NetError> {
                Err(NetError::TransportFull)
            }
        }
        let pt = PublicTranscript {
            state_root: [0u8; 32],
            receipt_root: [0u8; 32],
            efb_root: [0u8; 32],
            epoch: 0,
            halt_flag: false,
        };
        assert_eq!(
            publish_transcript_entry(&FailTransport, &pt),
            Err(NetError::TransportFull)
        );
    }
}
