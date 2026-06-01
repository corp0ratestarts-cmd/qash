// PAL-side clone protocol support modules.
//
// GENESIS_CONSTANTS.toml [clone_protocol] features implemented here:
//   bloom_filter_dedup = true    → dedup::ChunkRelayFilter
//   packet_compression = "LZ4"  → compression::{compress_chunk_payload, …}
//   cover_traffic = true         → cover_traffic::{CoverTrafficScheduler, …}
//   store_and_forward = true     → relay::StoreForwardBuffer
//   emergency_wipe_signal = true → wipe::WipeSignal
//   channels = [...]             → transport::{CloneTransport, ChunkFrame, …}
//
// Domain B only. None of these modules may influence Domain A state.

pub mod compression;
pub mod cover_traffic;
pub mod dedup;
pub mod manifest;
pub mod relay;
pub mod transport;
pub mod wipe;

pub use compression::{
    compress_chunk_payload, decompress_chunk_payload, is_compressed, CompressionError,
    MAX_DECOMPRESSED_BYTES,
};
pub use cover_traffic::{
    is_dummy_payload, make_dummy_payload, CoverTrafficScheduler, DEFAULT_INTERVAL_MS, DUMMY_MAGIC,
    DUMMY_PAYLOAD_BYTES,
};
pub use dedup::ChunkRelayFilter;
pub use relay::{
    BufferedChunk, RelayError, StoreForwardBuffer, MAX_BUFFERED_CHUNKS, MAX_EPOCH_AGE,
};
pub use transport::{
    crc16_ccitt, decode_ultrasonic_frame, encode_ultrasonic_frame, BleTransport, ChunkFrame,
    CloneTransport, FrameError, LoRaTransport, NfcTransport, QrTransport, TransportError,
    UltrasonicError, UltrasonicTransport, WifiDirectTransport, FRAME_VERSION,
    MAX_COMPRESSED_PAYLOAD, MAX_ULTRASONIC_PAYLOAD, SIG_BYTES, ULTRASONIC_SYNC,
};
pub use manifest::{
    compute_clone_manifest_root_pair, verify_clone_manifest_root_pair, CloneManifestError,
    ClonePackageManifest,
};
pub use wipe::{WipeError, WipeSignal, WIPE_MAGIC, WIPE_SIGNAL_BYTES, WIPE_VERSION};
