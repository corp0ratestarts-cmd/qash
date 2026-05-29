// Transport stubs for QR, NFC, BLE, WiFi-Direct, LoRa, Ultrasonic (spec §10.1).
//
// Each stub implements `CloneTransport` with the correct MTU and name, but
// returns `TransportError::NotAvailable` for send/recv until the platform
// HAL is wired.  The interface is stable; hardware integration is the only
// remaining step.
//
// MTUs match GENESIS_CONSTANTS.toml [clone_protocol] channel capacities and
// src/offline/clone.rs `max_chunk_bytes()`.
//
// BLE notes (spec §10.4):
//   Dual-role (peripheral + central simultaneously) is a hardware capability;
//   the BleTransport stub carries the `dual_role` flag but cannot enforce it
//   without a real BLE stack.  Advertisement payloads must NOT contain the
//   validator identity — only epoch counter + ephemeral session token.

use super::ultrasonic::MAX_ULTRASONIC_PAYLOAD;
use super::{CloneTransport, TransportError};

/// QR code transport stub.  MTU: 2048 bytes (QR capacity at ECC level M).
pub struct QrTransport;

impl CloneTransport for QrTransport {
    fn max_frame_bytes(&self) -> usize {
        2048
    }
    fn name(&self) -> &'static str {
        "QR_code"
    }

    fn send(&self, frame: &[u8]) -> Result<(), TransportError> {
        if frame.len() > self.max_frame_bytes() {
            return Err(TransportError::FrameTooLarge {
                max: self.max_frame_bytes(),
                got: frame.len(),
            });
        }
        Err(TransportError::NotAvailable)
    }

    fn recv(&self) -> Result<Option<Vec<u8>>, TransportError> {
        Err(TransportError::NotAvailable)
    }
}

/// NFC transport stub (ISO 14443 APDU).  MTU: 512 bytes.
pub struct NfcTransport;

impl CloneTransport for NfcTransport {
    fn max_frame_bytes(&self) -> usize {
        512
    }
    fn name(&self) -> &'static str {
        "NFC"
    }

    fn send(&self, frame: &[u8]) -> Result<(), TransportError> {
        if frame.len() > self.max_frame_bytes() {
            return Err(TransportError::FrameTooLarge {
                max: self.max_frame_bytes(),
                got: frame.len(),
            });
        }
        Err(TransportError::NotAvailable)
    }

    fn recv(&self) -> Result<Option<Vec<u8>>, TransportError> {
        Err(TransportError::NotAvailable)
    }
}

/// BLE 5.x transport stub.  MTU: 512 bytes.
///
/// `dual_role = true` means the device simultaneously acts as peripheral
/// (advertising) and central (scanning), per spec §10.4.
pub struct BleTransport {
    pub dual_role: bool,
}

impl BleTransport {
    pub fn new() -> Self {
        Self { dual_role: true }
    }
}

impl Default for BleTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl CloneTransport for BleTransport {
    fn max_frame_bytes(&self) -> usize {
        512
    }
    fn name(&self) -> &'static str {
        "BLE"
    }

    fn send(&self, frame: &[u8]) -> Result<(), TransportError> {
        if frame.len() > self.max_frame_bytes() {
            return Err(TransportError::FrameTooLarge {
                max: self.max_frame_bytes(),
                got: frame.len(),
            });
        }
        Err(TransportError::NotAvailable)
    }

    fn recv(&self) -> Result<Option<Vec<u8>>, TransportError> {
        Err(TransportError::NotAvailable)
    }
}

/// WiFi-Direct (IEEE 802.11 P2P) transport stub.  MTU: 65536 bytes.
pub struct WifiDirectTransport;

impl CloneTransport for WifiDirectTransport {
    fn max_frame_bytes(&self) -> usize {
        65536
    }
    fn name(&self) -> &'static str {
        "WiFi_Direct"
    }

    fn send(&self, frame: &[u8]) -> Result<(), TransportError> {
        if frame.len() > self.max_frame_bytes() {
            return Err(TransportError::FrameTooLarge {
                max: self.max_frame_bytes(),
                got: frame.len(),
            });
        }
        Err(TransportError::NotAvailable)
    }

    fn recv(&self) -> Result<Option<Vec<u8>>, TransportError> {
        Err(TransportError::NotAvailable)
    }
}

/// LoRa/LoRaWAN transport stub.  MTU: 255 bytes.
pub struct LoRaTransport;

impl CloneTransport for LoRaTransport {
    fn max_frame_bytes(&self) -> usize {
        255
    }
    fn name(&self) -> &'static str {
        "LoRa"
    }

    fn send(&self, frame: &[u8]) -> Result<(), TransportError> {
        if frame.len() > self.max_frame_bytes() {
            return Err(TransportError::FrameTooLarge {
                max: self.max_frame_bytes(),
                got: frame.len(),
            });
        }
        Err(TransportError::NotAvailable)
    }

    fn recv(&self) -> Result<Option<Vec<u8>>, TransportError> {
        Err(TransportError::NotAvailable)
    }
}

/// Ultrasonic FSK transport stub (spec §10.2).  MTU: 255 bytes (MAX_ULTRASONIC_PAYLOAD).
///
/// Carrier of last resort (transport priority 6): 24 kHz Mark / 24.5 kHz Space,
/// binary FSK at ≤ 1200 baud.  Physical layer (ADC/DAC, FSK demodulation) is
/// handled by the platform HAL; this stub holds the interface.
///
/// `send()` receives chunk payload bytes; the HAL wraps them in the §10.2
/// physical frame (SYNC || LEN || PAYLOAD || CRC16) before modulation.
pub struct UltrasonicTransport;

impl CloneTransport for UltrasonicTransport {
    fn max_frame_bytes(&self) -> usize {
        MAX_ULTRASONIC_PAYLOAD
    }

    fn name(&self) -> &'static str {
        "Ultrasonic"
    }

    fn send(&self, frame: &[u8]) -> Result<(), TransportError> {
        if frame.len() > self.max_frame_bytes() {
            return Err(TransportError::FrameTooLarge {
                max: self.max_frame_bytes(),
                got: frame.len(),
            });
        }
        Err(TransportError::NotAvailable)
    }

    fn recv(&self) -> Result<Option<Vec<u8>>, TransportError> {
        Err(TransportError::NotAvailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clone::transport::CloneTransport;

    #[test]
    fn all_stubs_have_nonzero_mtu() {
        let transports: &[&dyn CloneTransport] = &[
            &QrTransport,
            &NfcTransport,
            &BleTransport::new(),
            &WifiDirectTransport,
            &LoRaTransport,
            &UltrasonicTransport,
        ];
        for t in transports {
            assert!(t.max_frame_bytes() > 0, "{} has zero MTU", t.name());
        }
    }

    #[test]
    fn wifi_direct_has_largest_mtu() {
        assert!(WifiDirectTransport.max_frame_bytes() > QrTransport.max_frame_bytes());
        assert!(WifiDirectTransport.max_frame_bytes() > LoRaTransport.max_frame_bytes());
    }

    #[test]
    fn all_stubs_return_not_available() {
        let t = NfcTransport;
        assert_eq!(t.send(b"test"), Err(TransportError::NotAvailable));
        assert_eq!(t.recv(), Err(TransportError::NotAvailable));
    }

    #[test]
    fn stubs_reject_oversized_frames_before_not_available() {
        let t = LoRaTransport;
        let big = vec![0u8; t.max_frame_bytes() + 1];
        assert!(matches!(
            t.send(&big),
            Err(TransportError::FrameTooLarge { .. })
        ));
    }

    #[test]
    fn ble_defaults_to_dual_role() {
        let t = BleTransport::new();
        assert!(t.dual_role);
    }

    #[test]
    fn transport_names_match_genesis_constants() {
        assert_eq!(QrTransport.name(), "QR_code");
        assert_eq!(NfcTransport.name(), "NFC");
        assert_eq!(BleTransport::new().name(), "BLE");
        assert_eq!(WifiDirectTransport.name(), "WiFi_Direct");
        assert_eq!(LoRaTransport.name(), "LoRa");
        assert_eq!(UltrasonicTransport.name(), "Ultrasonic");
    }

    #[test]
    fn ultrasonic_mtu_matches_max_ultrasonic_payload() {
        assert_eq!(
            UltrasonicTransport.max_frame_bytes(),
            MAX_ULTRASONIC_PAYLOAD
        );
    }

    #[test]
    fn ultrasonic_is_lowest_mtu_transport() {
        // Priority 6 (carrier of last resort) — tightest bandwidth constraint.
        assert!(UltrasonicTransport.max_frame_bytes() <= LoRaTransport.max_frame_bytes());
        assert!(UltrasonicTransport.max_frame_bytes() <= NfcTransport.max_frame_bytes());
    }

    #[test]
    fn ultrasonic_rejects_oversized_frame() {
        let big = vec![0u8; UltrasonicTransport.max_frame_bytes() + 1];
        assert!(matches!(
            UltrasonicTransport.send(&big),
            Err(TransportError::FrameTooLarge { .. })
        ));
    }
}
