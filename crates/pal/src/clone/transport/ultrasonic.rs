// Ultrasonic transport physical framing — spec §10.2.
//
// Physical parameters:
//   Carrier:     24 kHz (Mark), 24.5 kHz (Space)
//   Modulation:  Binary FSK
//   Symbol rate: ≤ 1200 baud (300/600/1200 admitted)
//   Max payload: 255 bytes per frame (constrained by 2-byte LEN field)
//
// Wire frame (byte level; FSK modulation is hardware-handled):
//   [SYNC 4][LEN 2 LE][PAYLOAD ≤ 255][CRC16 2]
//   SYNC = 0x55 0x55 0xAA 0x55
//   CRC16 = CRC-CCITT (poly 0x1021, init 0xFFFF) over LEN || PAYLOAD
//
// The ultrasonic transport is the carrier of last resort (transport priority 6).
// It is used in RF-denied environments: Faraday cages, jamming zones.
//
// This module handles framing only.  Physical layer (ADC/DAC, FSK demodulation)
// is left to the platform HAL; call send_frame() / recv_frame() through the
// OS audio interface or dedicated piezoelectric driver.

/// Sync preamble for ultrasonic FSK synchronisation.
pub const SYNC: [u8; 4] = [0x55, 0x55, 0xAA, 0x55];

/// Maximum payload bytes per ultrasonic frame (limited by 2-byte LEN field; ≤ 255).
pub const MAX_ULTRASONIC_PAYLOAD: usize = 255;

/// Total minimum frame bytes (SYNC + LEN + CRC, no payload).
pub const MIN_FRAME_BYTES: usize = 4 + 2 + 2; // = 8

/// Error type for ultrasonic frame operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UltrasonicError {
    PayloadTooLarge(usize),
    FrameTooShort,
    BadSync,
    CrcMismatch,
    LengthMismatch,
    TrailingBytes,
}

impl std::fmt::Display for UltrasonicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UltrasonicError::PayloadTooLarge(n) => write!(f, "payload too large: {n}"),
            UltrasonicError::FrameTooShort => write!(f, "frame too short"),
            UltrasonicError::BadSync => write!(f, "bad sync preamble"),
            UltrasonicError::CrcMismatch => write!(f, "CRC16 mismatch"),
            UltrasonicError::LengthMismatch => write!(f, "LEN field does not match payload"),
            UltrasonicError::TrailingBytes => write!(f, "trailing bytes after CRC"),
        }
    }
}

/// Encode `payload` into an ultrasonic physical frame.
///
/// Returns the raw byte sequence to hand to the FSK modulator.
/// `payload.len()` must be ≤ MAX_ULTRASONIC_PAYLOAD (255).
pub fn encode_frame(payload: &[u8]) -> Result<Vec<u8>, UltrasonicError> {
    if payload.len() > MAX_ULTRASONIC_PAYLOAD {
        return Err(UltrasonicError::PayloadTooLarge(payload.len()));
    }
    let len = payload.len() as u16;
    let crc = crc16_ccitt(&len.to_le_bytes(), payload);

    let mut out = Vec::with_capacity(4 + 2 + payload.len() + 2);
    out.extend_from_slice(&SYNC);
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(payload);
    out.extend_from_slice(&crc.to_le_bytes());
    Ok(out)
}

/// Decode an ultrasonic physical frame from raw bytes.
///
/// Returns the payload on success. Validates sync preamble and CRC16.
pub fn decode_frame(bytes: &[u8]) -> Result<Vec<u8>, UltrasonicError> {
    if bytes.len() < MIN_FRAME_BYTES {
        return Err(UltrasonicError::FrameTooShort);
    }
    if bytes[..4] != SYNC {
        return Err(UltrasonicError::BadSync);
    }
    let len = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
    if len > MAX_ULTRASONIC_PAYLOAD {
        return Err(UltrasonicError::PayloadTooLarge(len));
    }
    let payload_end = 6 + len;
    let frame_end = payload_end + 2;
    if bytes.len() < frame_end {
        return Err(UltrasonicError::LengthMismatch);
    }
    if bytes.len() > frame_end {
        return Err(UltrasonicError::TrailingBytes);
    }
    let payload = &bytes[6..payload_end];
    let expected_crc = crc16_ccitt(&(len as u16).to_le_bytes(), payload);
    let actual_crc = u16::from_le_bytes([bytes[payload_end], bytes[payload_end + 1]]);
    if expected_crc != actual_crc {
        return Err(UltrasonicError::CrcMismatch);
    }
    Ok(payload.to_vec())
}

/// CRC-CCITT (poly = 0x1021, init = 0xFFFF) over `prefix` then `data`.
///
/// Spec §10.2: CRC16 = CRC-CCITT over LEN || PAYLOAD.
pub fn crc16_ccitt(prefix: &[u8], data: &[u8]) -> u16 {
    const POLY: u16 = 0x1021;
    let mut crc: u16 = 0xFFFF;
    for &byte in prefix.iter().chain(data.iter()) {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ POLY;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_empty_payload() {
        let frame = encode_frame(b"").unwrap();
        let payload = decode_frame(&frame).unwrap();
        assert!(payload.is_empty());
    }

    #[test]
    fn roundtrip_small_payload() {
        let payload = b"QASH ultrasonic test";
        let frame = encode_frame(payload).unwrap();
        let decoded = decode_frame(&frame).unwrap();
        assert_eq!(decoded, payload);
    }

    #[test]
    fn roundtrip_max_payload() {
        let payload = vec![0xA5u8; MAX_ULTRASONIC_PAYLOAD];
        let frame = encode_frame(&payload).unwrap();
        let decoded = decode_frame(&frame).unwrap();
        assert_eq!(decoded, payload);
    }

    #[test]
    fn frame_has_sync_preamble() {
        let frame = encode_frame(b"hello").unwrap();
        assert_eq!(&frame[..4], &SYNC);
    }

    #[test]
    fn encode_rejects_oversized_payload() {
        let big = vec![0u8; MAX_ULTRASONIC_PAYLOAD + 1];
        assert_eq!(
            encode_frame(&big).unwrap_err(),
            UltrasonicError::PayloadTooLarge(MAX_ULTRASONIC_PAYLOAD + 1)
        );
    }

    #[test]
    fn decode_rejects_bad_sync() {
        let mut frame = encode_frame(b"data").unwrap();
        frame[2] ^= 0xFF;
        assert_eq!(decode_frame(&frame).unwrap_err(), UltrasonicError::BadSync);
    }

    #[test]
    fn decode_rejects_crc_mismatch() {
        let mut frame = encode_frame(b"data").unwrap();
        let last = frame.len() - 1;
        frame[last] ^= 0x01;
        assert_eq!(
            decode_frame(&frame).unwrap_err(),
            UltrasonicError::CrcMismatch
        );
    }

    #[test]
    fn decode_rejects_short_frame() {
        assert_eq!(
            decode_frame(&[0u8; 4]).unwrap_err(),
            UltrasonicError::FrameTooShort
        );
    }

    #[test]
    fn decode_rejects_trailing_bytes() {
        let mut frame = encode_frame(b"data").unwrap();
        frame.push(0x00); // one extra byte after CRC
        assert_eq!(
            decode_frame(&frame).unwrap_err(),
            UltrasonicError::TrailingBytes
        );
    }

    #[test]
    fn crc16_known_vector() {
        // CRC-CCITT of b"123456789" = 0x29B1 (standard test vector).
        let crc = crc16_ccitt(b"", b"123456789");
        assert_eq!(crc, 0x29B1);
    }

    #[test]
    fn distinct_payloads_produce_distinct_frames() {
        let a = encode_frame(b"payload-a").unwrap();
        let b = encode_frame(b"payload-b").unwrap();
        assert_ne!(a, b);
    }
}
