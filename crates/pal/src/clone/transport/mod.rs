// Clone protocol transport layer — spec §10.1.
//
// This module defines the CloneTransport trait (§10.1 interface contract) and
// provides:
//   frame.rs     — §10.7 chunk wire format (VERSION/EPOCH/SIG header)
//   ultrasonic.rs — §10.2 FSK/CRC physical framing (fully spec'd)
//   stubs.rs     — §10.1 transport stubs for QR, NFC, BLE, WiFi-Direct, LoRa, Ultrasonic
//                  (interface complete; hardware integration deferred)
//
// Transport priority order (§10.3, highest-bandwidth first):
//   1. WiFi_Direct
//   2. BLE (dual-role: concurrent peripheral + central, §10.4)
//   3. NFC
//   4. LoRa
//   5. QR_code
//   6. Ultrasonic (carrier of last resort)
//
// Domain B only.

pub mod frame;
pub mod stubs;
pub mod ultrasonic;

pub use frame::{ChunkFrame, FrameError, FRAME_VERSION, MAX_COMPRESSED_PAYLOAD, SIG_BYTES};
pub use stubs::{
    BleTransport, LoRaTransport, NfcTransport, QrTransport, UltrasonicTransport, WifiDirectTransport,
};
pub use ultrasonic::{
    crc16_ccitt, decode_frame as decode_ultrasonic_frame, encode_frame as encode_ultrasonic_frame,
    UltrasonicError, MAX_ULTRASONIC_PAYLOAD, SYNC as ULTRASONIC_SYNC,
};

/// Error type for transport send/receive operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    /// Hardware not available or not initialised.
    NotAvailable,
    /// Frame too large for this channel's MTU.
    FrameTooLarge { max: usize, got: usize },
    /// Underlying I/O error (driver-level; opaque string for Domain B use).
    Io(String),
    /// Channel-specific framing error.
    Framing(String),
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportError::NotAvailable => write!(f, "transport not available"),
            TransportError::FrameTooLarge { max, got } => {
                write!(f, "frame too large: max {max}, got {got}")
            }
            TransportError::Io(msg) => write!(f, "I/O error: {msg}"),
            TransportError::Framing(msg) => write!(f, "framing error: {msg}"),
        }
    }
}

/// Clone transport channel interface (spec §10.1).
///
/// All admitted transports (QR, NFC, BLE, WiFi-Direct, LoRa, Ultrasonic)
/// implement this trait.  The consensus layer never observes which transport
/// delivered a given chunk.
pub trait CloneTransport {
    /// Maximum bytes per send call for this transport.
    fn max_frame_bytes(&self) -> usize;

    /// Send `frame` bytes over this transport channel.
    ///
    /// The caller is responsible for chunking to `max_frame_bytes()` before
    /// calling. Returns `FrameTooLarge` if `frame.len() > max_frame_bytes()`.
    fn send(&self, frame: &[u8]) -> Result<(), TransportError>;

    /// Attempt to receive one frame.  Returns `None` if nothing is available.
    fn recv(&self) -> Result<Option<Vec<u8>>, TransportError>;

    /// Human-readable transport name (for diagnostics only, never Domain A).
    fn name(&self) -> &'static str;
}
