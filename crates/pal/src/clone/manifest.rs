//! Clone/export package manifest with independent dual-root all-of verification.
//!
//! One `AllOfHashPair32` root per clone/export package manifest.
//! No all-of root is added per individual chunk — the manifest covers the
//! package as a whole.
//!
//! Domain B only. Does not influence Domain A state.

use crate::crypto::dual_hash::{allof_hash_pair_32, verify_allof_hash_pair_32, AllOfHashPair32};

/// A clone/export package manifest binding chunk hashes and metadata to an
/// independent dual-root all-of pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClonePackageManifest {
    pub version: u8,
    pub transport_tag: [u8; 8],
    pub epoch: u64,
    pub chunk_count: u16,
    pub chunk_hashes: Vec<[u8; 32]>,
    pub compression_tag: [u8; 4],
    pub manifest_root_pair: AllOfHashPair32,
}

/// Errors for clone/export manifest construction and validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloneManifestError {
    EmptyManifest,
    ChunkCountMismatch,
    TooManyChunks,
}

impl ClonePackageManifest {
    /// Construct a manifest, computing the all-of root pair from the provided data.
    pub fn new(
        version: u8,
        transport_tag: [u8; 8],
        epoch: u64,
        chunk_hashes: Vec<[u8; 32]>,
        compression_tag: [u8; 4],
    ) -> Result<Self, CloneManifestError> {
        if chunk_hashes.is_empty() {
            return Err(CloneManifestError::EmptyManifest);
        }
        if chunk_hashes.len() > u16::MAX as usize {
            return Err(CloneManifestError::TooManyChunks);
        }
        let chunk_count = chunk_hashes.len() as u16;
        let mut manifest = Self {
            version,
            transport_tag,
            epoch,
            chunk_count,
            chunk_hashes,
            compression_tag,
            manifest_root_pair: AllOfHashPair32 {
                sha3_512_32: [0u8; 32],
                blake3_32: [0u8; 32],
            },
        };
        manifest.manifest_root_pair = compute_clone_manifest_root_pair(&manifest)?;
        Ok(manifest)
    }
}

/// Compute the all-of root pair for a clone/export manifest.
///
/// Transcript:
/// - context = `b"qash-clone-manifest-v1"`
/// - salt    = `epoch.to_le_bytes()`
/// - data    = `version || transport_tag || chunk_count_le || chunk_hashes_flat || compression_tag`
///
/// `manifest_root_pair` is NOT included in the transcript.
pub fn compute_clone_manifest_root_pair(
    manifest: &ClonePackageManifest,
) -> Result<AllOfHashPair32, CloneManifestError> {
    validate_clone_manifest_shape(manifest)?;

    let mut data = Vec::with_capacity(1 + 8 + 2 + manifest.chunk_hashes.len() * 32 + 4);
    data.push(manifest.version);
    data.extend_from_slice(&manifest.transport_tag);
    data.extend_from_slice(&manifest.chunk_count.to_le_bytes());
    for h in &manifest.chunk_hashes {
        data.extend_from_slice(h);
    }
    data.extend_from_slice(&manifest.compression_tag);

    Ok(allof_hash_pair_32(
        b"qash-clone-manifest-v1",
        &manifest.epoch.to_le_bytes(),
        &data,
    ))
}

/// Verify the `manifest_root_pair` of a `ClonePackageManifest`.
///
/// Returns `true` only when both SHA3-512 and BLAKE3 arms independently match.
pub fn verify_clone_manifest_root_pair(manifest: &ClonePackageManifest) -> bool {
    if validate_clone_manifest_shape(manifest).is_err() {
        return false;
    }

    let mut data = Vec::with_capacity(1 + 8 + 2 + manifest.chunk_hashes.len() * 32 + 4);
    data.push(manifest.version);
    data.extend_from_slice(&manifest.transport_tag);
    data.extend_from_slice(&manifest.chunk_count.to_le_bytes());
    for h in &manifest.chunk_hashes {
        data.extend_from_slice(h);
    }
    data.extend_from_slice(&manifest.compression_tag);

    verify_allof_hash_pair_32(
        &manifest.manifest_root_pair,
        b"qash-clone-manifest-v1",
        &manifest.epoch.to_le_bytes(),
        &data,
    )
}

fn validate_clone_manifest_shape(
    manifest: &ClonePackageManifest,
) -> Result<(), CloneManifestError> {
    if manifest.chunk_hashes.is_empty() {
        return Err(CloneManifestError::EmptyManifest);
    }
    if manifest.chunk_hashes.len() > u16::MAX as usize {
        return Err(CloneManifestError::TooManyChunks);
    }
    if manifest.chunk_hashes.len() != manifest.chunk_count as usize {
        return Err(CloneManifestError::ChunkCountMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_hashes(n: usize) -> Vec<[u8; 32]> {
        (0..n)
            .map(|i| {
                let mut h = [0u8; 32];
                h[..4].copy_from_slice(&(i as u32).to_le_bytes());
                h
            })
            .collect()
    }

    fn sample_manifest() -> ClonePackageManifest {
        ClonePackageManifest::new(1, *b"TAG00001", 42, make_hashes(3), *b"LZ40")
            .expect("valid manifest")
    }

    #[test]
    fn clone_manifest_accepts_exact_root_pair() {
        let m = sample_manifest();
        assert!(verify_clone_manifest_root_pair(&m));
    }

    #[test]
    fn clone_manifest_rejects_modified_sha3_root() {
        let mut m = sample_manifest();
        m.manifest_root_pair.sha3_512_32 = [0u8; 32];
        assert!(!verify_clone_manifest_root_pair(&m));
    }

    #[test]
    fn clone_manifest_rejects_modified_blake3_root() {
        let mut m = sample_manifest();
        m.manifest_root_pair.blake3_32 = [0u8; 32];
        assert!(!verify_clone_manifest_root_pair(&m));
    }

    #[test]
    fn clone_manifest_root_changes_when_chunk_hash_changes() {
        let m1 = sample_manifest();
        let mut hashes = make_hashes(3);
        hashes[1] = [0xFFu8; 32];
        let m2 = ClonePackageManifest::new(1, *b"TAG00001", 42, hashes, *b"LZ40").unwrap();
        assert_ne!(m1.manifest_root_pair, m2.manifest_root_pair);
    }

    #[test]
    fn clone_manifest_root_changes_when_chunk_order_changes() {
        let mut hashes = make_hashes(3);
        let m1 = ClonePackageManifest::new(1, *b"TAG00001", 42, hashes.clone(), *b"LZ40").unwrap();
        hashes.swap(0, 2);
        let m2 = ClonePackageManifest::new(1, *b"TAG00001", 42, hashes, *b"LZ40").unwrap();
        assert_ne!(m1.manifest_root_pair, m2.manifest_root_pair);
    }

    #[test]
    fn clone_manifest_root_changes_when_metadata_changes() {
        let m1 = sample_manifest();
        let m2 = ClonePackageManifest::new(2, *b"TAG00001", 42, make_hashes(3), *b"LZ40").unwrap();
        assert_ne!(m1.manifest_root_pair, m2.manifest_root_pair);
    }

    #[test]
    fn clone_manifest_rejects_chunk_count_mismatch() {
        let mut m = sample_manifest();
        m.chunk_count = 99;
        assert!(!verify_clone_manifest_root_pair(&m));
    }

    #[test]
    fn clone_manifest_rejects_empty_manifest() {
        assert_eq!(
            ClonePackageManifest::new(1, *b"TAG00001", 42, vec![], *b"LZ40"),
            Err(CloneManifestError::EmptyManifest)
        );
    }
}
