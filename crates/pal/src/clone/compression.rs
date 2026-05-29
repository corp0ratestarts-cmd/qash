// LZ4 chunk payload compression for clone protocol.
//
// GENESIS_CONSTANTS.toml [clone_protocol]: packet_compression = "LZ4"
//
// Applied to clone-chunk payloads BEFORE cascade signing. The compressor
// reduces chunk size for bandwidth-constrained channels (LoRa, Ultrasonic).
// Domain B only — compression output never flows into Domain A.
//
// Wire format (compressed frame):
//   magic(4) || uncompressed_len_le4 || compressed_bytes
//
// Callers must verify uncompressed_len does not exceed MAX_CHUNK_BYTES before
// decompressing to prevent decompression-bomb DoS.

use std::io;

/// Frame magic for compressed clone chunk payloads.
const COMPRESSED_MAGIC: &[u8; 4] = b"LZ4\0";

/// Maximum allowed decompressed size (64 KiB — WiFi Direct chunk max).
pub const MAX_DECOMPRESSED_BYTES: usize = 65536;

/// Compress `payload` using LZ4 block format and prepend the wire header.
///
/// Returns `Err` only if the payload exceeds `MAX_DECOMPRESSED_BYTES`.
pub fn compress_chunk_payload(payload: &[u8]) -> Result<Vec<u8>, CompressionError> {
    if payload.len() > MAX_DECOMPRESSED_BYTES {
        return Err(CompressionError::PayloadTooLarge(payload.len()));
    }
    let compressed = lz4_flex::compress_prepend_size(payload);
    let mut out = Vec::with_capacity(4 + 4 + compressed.len());
    out.extend_from_slice(COMPRESSED_MAGIC);
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(&compressed);
    Ok(out)
}

/// Decompress a frame produced by `compress_chunk_payload`.
///
/// Validates magic, decompressed-size bound, and LZ4 integrity.
pub fn decompress_chunk_payload(frame: &[u8]) -> Result<Vec<u8>, CompressionError> {
    if frame.len() < 8 {
        return Err(CompressionError::InvalidFrame("frame too short"));
    }
    if &frame[..4] != COMPRESSED_MAGIC {
        return Err(CompressionError::InvalidFrame("bad magic"));
    }
    let uncompressed_len = u32::from_le_bytes([frame[4], frame[5], frame[6], frame[7]]) as usize;
    if uncompressed_len > MAX_DECOMPRESSED_BYTES {
        return Err(CompressionError::PayloadTooLarge(uncompressed_len));
    }
    let decompressed = lz4_flex::decompress_size_prepended(&frame[8..])
        .map_err(|e| CompressionError::Lz4(e.to_string()))?;
    if decompressed.len() != uncompressed_len {
        return Err(CompressionError::InvalidFrame(
            "length mismatch after decompression",
        ));
    }
    Ok(decompressed)
}

/// True if `frame` begins with the compressed-payload magic.
pub fn is_compressed(frame: &[u8]) -> bool {
    frame.starts_with(COMPRESSED_MAGIC)
}

/// Errors from chunk payload compression / decompression.
#[derive(Debug)]
pub enum CompressionError {
    PayloadTooLarge(usize),
    InvalidFrame(&'static str),
    Lz4(String),
}

impl std::fmt::Display for CompressionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompressionError::PayloadTooLarge(n) => write!(f, "payload too large: {n} bytes"),
            CompressionError::InvalidFrame(msg) => write!(f, "invalid frame: {msg}"),
            CompressionError::Lz4(msg) => write!(f, "LZ4 error: {msg}"),
        }
    }
}

impl From<CompressionError> for io::Error {
    fn from(e: CompressionError) -> Self {
        io::Error::new(io::ErrorKind::InvalidData, e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_small_payload() {
        let payload = b"QASH clone chunk payload for test";
        let frame = compress_chunk_payload(payload).unwrap();
        let recovered = decompress_chunk_payload(&frame).unwrap();
        assert_eq!(recovered, payload);
    }

    #[test]
    fn roundtrip_empty_payload() {
        let frame = compress_chunk_payload(b"").unwrap();
        let recovered = decompress_chunk_payload(&frame).unwrap();
        assert_eq!(recovered, b"");
    }

    #[test]
    fn roundtrip_repetitive_payload_compresses_well() {
        let payload = vec![0xABu8; 4096];
        let frame = compress_chunk_payload(&payload).unwrap();
        // LZ4 should compress repetitive data significantly.
        assert!(
            frame.len() < payload.len(),
            "LZ4 did not compress repetitive data"
        );
        let recovered = decompress_chunk_payload(&frame).unwrap();
        assert_eq!(recovered, payload);
    }

    #[test]
    fn is_compressed_detects_magic() {
        let frame = compress_chunk_payload(b"hello").unwrap();
        assert!(is_compressed(&frame));
        assert!(!is_compressed(b"raw-bytes"));
        assert!(!is_compressed(b""));
    }

    #[test]
    fn decompress_rejects_bad_magic() {
        let mut frame = compress_chunk_payload(b"data").unwrap();
        frame[0] ^= 0xFF;
        assert!(matches!(
            decompress_chunk_payload(&frame),
            Err(CompressionError::InvalidFrame(_))
        ));
    }

    #[test]
    fn decompress_rejects_too_short_frame() {
        assert!(matches!(
            decompress_chunk_payload(&[0u8; 4]),
            Err(CompressionError::InvalidFrame(_))
        ));
    }

    #[test]
    fn compress_rejects_oversized_payload() {
        let big = vec![0u8; MAX_DECOMPRESSED_BYTES + 1];
        assert!(matches!(
            compress_chunk_payload(&big),
            Err(CompressionError::PayloadTooLarge(_))
        ));
    }

    #[test]
    fn decompress_rejects_oversized_length_field() {
        // Craft a frame with a length field exceeding MAX_DECOMPRESSED_BYTES.
        let mut frame = vec![0u8; 16];
        frame[..4].copy_from_slice(COMPRESSED_MAGIC);
        // uncompressed_len = MAX + 1
        let bad_len = (MAX_DECOMPRESSED_BYTES + 1) as u32;
        frame[4..8].copy_from_slice(&bad_len.to_le_bytes());
        assert!(matches!(
            decompress_chunk_payload(&frame),
            Err(CompressionError::PayloadTooLarge(_))
        ));
    }
}
