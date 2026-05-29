// PAL-side clone protocol support modules.
//
// GENESIS_CONSTANTS.toml [clone_protocol] features implemented here:
//   bloom_filter_dedup = true  → dedup::ChunkRelayFilter
//   packet_compression = "LZ4" → compression::{compress_chunk_payload, decompress_chunk_payload}
//
// Domain B only. None of these modules may influence Domain A state.

pub mod compression;
pub mod dedup;

pub use compression::{
    compress_chunk_payload, decompress_chunk_payload, is_compressed, CompressionError,
    MAX_DECOMPRESSED_BYTES,
};
pub use dedup::ChunkRelayFilter;
