//! Release/evidence bundle manifest with independent dual-root all-of verification.
//!
//! Low-frequency, high-value evidence binding for release manifests and audit
//! bundles. The all-of root pair is stored separately from the manifest
//! transcript so neither field poisons the other.
//!
//! Domain B only. Does not alter genesis constants or Domain A state.

use crate::crypto::dual_hash::{allof_hash_pair_32, verify_allof_hash_pair_32, AllOfHashPair32};

/// Manifest data for a release/evidence bundle.
///
/// `commit_sha` is the SHA-256 digest of the canonical Git commit identifier
/// bytes. It serves as the transcript salt, binding this record to a specific
/// repository snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceBundleManifest {
    /// SHA-256 digest of the canonical Git commit identifier bytes.
    pub commit_sha: [u8; 32],
    pub genesis_constants_hash: [u8; 32],
    pub artifact_count: u32,
    pub artifact_hashes: Vec<[u8; 32]>,
    pub proof_coverage_hash: [u8; 32],
    pub traceability_hash: [u8; 32],
}

/// An evidence bundle record binding a manifest to an all-of root pair.
///
/// The `root_pair` is stored outside the transcript so the pair itself is
/// never hashed into its own derivation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceBundleRecord {
    pub manifest: EvidenceBundleManifest,
    pub root_pair: AllOfHashPair32,
}

/// Errors for evidence bundle construction and validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceBundleError {
    EmptyArtifacts,
    ArtifactCountMismatch,
    TooManyArtifacts,
}

impl EvidenceBundleRecord {
    /// Construct an evidence bundle record from a manifest, computing the root pair.
    pub fn new(manifest: EvidenceBundleManifest) -> Result<Self, EvidenceBundleError> {
        validate_evidence_bundle_manifest(&manifest)?;
        let root_pair = compute_evidence_bundle_root_pair(&manifest)?;
        Ok(Self {
            manifest,
            root_pair,
        })
    }

    /// Verify the stored root pair against the manifest content.
    pub fn verify(&self) -> bool {
        verify_evidence_bundle_root_pair(&self.manifest, &self.root_pair)
    }
}

/// Compute the all-of root pair for an evidence bundle manifest.
///
/// Transcript:
/// - context = `b"qash-evidence-bundle-v1"`
/// - salt    = `manifest.commit_sha`
/// - data    = `genesis_constants_hash || artifact_count_le || artifact_hashes_flat
///              || proof_coverage_hash || traceability_hash`
///
/// `root_pair` is NOT included in the transcript.
pub fn compute_evidence_bundle_root_pair(
    manifest: &EvidenceBundleManifest,
) -> Result<AllOfHashPair32, EvidenceBundleError> {
    validate_evidence_bundle_manifest(manifest)?;

    let mut data = Vec::with_capacity(32 + 4 + manifest.artifact_hashes.len() * 32 + 32 + 32);
    data.extend_from_slice(&manifest.genesis_constants_hash);
    data.extend_from_slice(&manifest.artifact_count.to_le_bytes());
    for h in &manifest.artifact_hashes {
        data.extend_from_slice(h);
    }
    data.extend_from_slice(&manifest.proof_coverage_hash);
    data.extend_from_slice(&manifest.traceability_hash);

    Ok(allof_hash_pair_32(
        b"qash-evidence-bundle-v1",
        &manifest.commit_sha,
        &data,
    ))
}

/// Verify an `AllOfHashPair32` against an evidence bundle manifest.
///
/// Returns `true` only when both SHA3-512 and BLAKE3 arms independently match.
pub fn verify_evidence_bundle_root_pair(
    manifest: &EvidenceBundleManifest,
    pair: &AllOfHashPair32,
) -> bool {
    if validate_evidence_bundle_manifest(manifest).is_err() {
        return false;
    }

    let mut data = Vec::with_capacity(32 + 4 + manifest.artifact_hashes.len() * 32 + 32 + 32);
    data.extend_from_slice(&manifest.genesis_constants_hash);
    data.extend_from_slice(&manifest.artifact_count.to_le_bytes());
    for h in &manifest.artifact_hashes {
        data.extend_from_slice(h);
    }
    data.extend_from_slice(&manifest.proof_coverage_hash);
    data.extend_from_slice(&manifest.traceability_hash);

    verify_allof_hash_pair_32(
        pair,
        b"qash-evidence-bundle-v1",
        &manifest.commit_sha,
        &data,
    )
}

fn validate_evidence_bundle_manifest(
    manifest: &EvidenceBundleManifest,
) -> Result<(), EvidenceBundleError> {
    if manifest.artifact_hashes.is_empty() {
        return Err(EvidenceBundleError::EmptyArtifacts);
    }
    if manifest.artifact_hashes.len() > u32::MAX as usize {
        return Err(EvidenceBundleError::TooManyArtifacts);
    }
    if manifest.artifact_hashes.len() != manifest.artifact_count as usize {
        return Err(EvidenceBundleError::ArtifactCountMismatch);
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

    fn test_hash(seed: u8) -> [u8; 32] {
        core::array::from_fn(|i| seed.wrapping_add(i as u8))
    }

    fn sample_manifest() -> EvidenceBundleManifest {
        let hashes = make_hashes(3);
        EvidenceBundleManifest {
            commit_sha: test_hash(0x01),
            genesis_constants_hash: test_hash(0x02),
            artifact_count: 3,
            artifact_hashes: hashes,
            proof_coverage_hash: test_hash(0x03),
            traceability_hash: test_hash(0x04),
        }
    }

    #[test]
    fn evidence_bundle_accepts_exact_root_pair() {
        let record = EvidenceBundleRecord::new(sample_manifest()).unwrap();
        assert!(record.verify());
    }

    #[test]
    fn evidence_bundle_rejects_modified_sha3_root() {
        let mut record = EvidenceBundleRecord::new(sample_manifest()).unwrap();
        record.root_pair.sha3_512_32 = [0u8; 32];
        assert!(!record.verify());
    }

    #[test]
    fn evidence_bundle_rejects_modified_blake3_root() {
        let mut record = EvidenceBundleRecord::new(sample_manifest()).unwrap();
        record.root_pair.blake3_32 = [0u8; 32];
        assert!(!record.verify());
    }

    #[test]
    fn evidence_bundle_root_changes_when_artifact_hash_changes() {
        let mut m2 = sample_manifest();
        m2.artifact_hashes[1] = [0xFFu8; 32];
        let r1 = EvidenceBundleRecord::new(sample_manifest()).unwrap();
        let r2 = EvidenceBundleRecord::new(m2).unwrap();
        assert_ne!(r1.root_pair, r2.root_pair);
    }

    #[test]
    fn evidence_bundle_root_changes_when_artifact_order_changes() {
        let m1 = sample_manifest();
        let mut m2 = sample_manifest();
        m2.artifact_hashes.swap(0, 2);
        let r1 = EvidenceBundleRecord::new(m1).unwrap();
        let r2 = EvidenceBundleRecord::new(m2).unwrap();
        assert_ne!(r1.root_pair, r2.root_pair);
    }

    #[test]
    fn evidence_bundle_root_changes_when_traceability_hash_changes() {
        let m1 = sample_manifest();
        let mut m2 = sample_manifest();
        m2.traceability_hash = [0xFFu8; 32];
        let r1 = EvidenceBundleRecord::new(m1).unwrap();
        let r2 = EvidenceBundleRecord::new(m2).unwrap();
        assert_ne!(r1.root_pair, r2.root_pair);
    }

    #[test]
    fn evidence_bundle_rejects_artifact_count_mismatch() {
        let mut m = sample_manifest();
        m.artifact_count = 99;
        assert_eq!(
            EvidenceBundleRecord::new(m),
            Err(EvidenceBundleError::ArtifactCountMismatch)
        );
    }

    #[test]
    fn evidence_bundle_rejects_empty_artifacts() {
        let mut m = sample_manifest();
        m.artifact_hashes = vec![];
        m.artifact_count = 0;
        assert_eq!(
            EvidenceBundleRecord::new(m),
            Err(EvidenceBundleError::EmptyArtifacts)
        );
    }
}
