//! Domain B MVP demonstrator types.
//!
//! These types support the offline incident receipt commit demonstrator. They
//! are not genesis-admitted Domain A transaction types.

use sha3::{Digest, Sha3_256};

pub const TX_MVP_RECEIPT_COMMIT_VERSION: u32 = 0x4d565001;
pub const TX_MVP_RECEIPT_COMMIT_BYTES: usize = 140;
pub const TX_MVP_PUBLIC_EXPORT_BYTES: usize = 140;

pub const TX_MVP_RECEIPT_COMMIT_DOMAIN_TAG: [u8; 32] = [
    b'Q', b'A', b'S', b'H', b'-', b'M', b'V', b'P', b'-', b'R', b'E', b'C', b'E', b'I', b'P',
    b'T', b'-', b'C', b'O', b'M', b'M', b'I', b'T', 0, 0, 0, 0, 0, 0, 0, 0, 1,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TxMvpReceiptCommit {
    pub version: u32,
    pub epoch: u64,
    pub nonce: [u8; 32],
    pub payload_commitment: [u8; 32],
    pub disclosure_key_commitment: [u8; 32],
    pub domain_tag: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TxMvpReceiptCommitPublicExport {
    pub version: u32,
    pub epoch: u64,
    pub tx_commitment: [u8; 32],
    pub nonce_commitment: [u8; 32],
    pub payload_commitment: [u8; 32],
    pub disclosure_key_commitment: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxMvpReceiptCommitError {
    InvalidVersion,
    InvalidDomainTag,
    InvalidLength,
    DuplicateEpochNonce,
}

impl TxMvpReceiptCommit {
    pub fn new(
        epoch: u64,
        nonce: [u8; 32],
        payload_commitment: [u8; 32],
        disclosure_key_commitment: [u8; 32],
    ) -> Self {
        Self {
            version: TX_MVP_RECEIPT_COMMIT_VERSION,
            epoch,
            nonce,
            payload_commitment,
            disclosure_key_commitment,
            domain_tag: TX_MVP_RECEIPT_COMMIT_DOMAIN_TAG,
        }
    }

    pub fn validate(&self) -> Result<(), TxMvpReceiptCommitError> {
        if self.version != TX_MVP_RECEIPT_COMMIT_VERSION {
            return Err(TxMvpReceiptCommitError::InvalidVersion);
        }
        if self.domain_tag != TX_MVP_RECEIPT_COMMIT_DOMAIN_TAG {
            return Err(TxMvpReceiptCommitError::InvalidDomainTag);
        }
        Ok(())
    }

    pub fn validate_epoch_nonce_unused<'a>(
        &self,
        prior: impl IntoIterator<Item = &'a TxMvpReceiptCommit>,
    ) -> Result<(), TxMvpReceiptCommitError> {
        self.validate()?;
        for existing in prior {
            if existing.epoch == self.epoch && existing.nonce == self.nonce {
                return Err(TxMvpReceiptCommitError::DuplicateEpochNonce);
            }
        }
        Ok(())
    }

    pub fn encode(&self) -> Result<[u8; TX_MVP_RECEIPT_COMMIT_BYTES], TxMvpReceiptCommitError> {
        self.validate()?;
        let mut out = [0u8; TX_MVP_RECEIPT_COMMIT_BYTES];
        let mut pos = 0;
        out[pos..pos + 4].copy_from_slice(&self.version.to_le_bytes());
        pos += 4;
        out[pos..pos + 8].copy_from_slice(&self.epoch.to_le_bytes());
        pos += 8;
        out[pos..pos + 32].copy_from_slice(&self.nonce);
        pos += 32;
        out[pos..pos + 32].copy_from_slice(&self.payload_commitment);
        pos += 32;
        out[pos..pos + 32].copy_from_slice(&self.disclosure_key_commitment);
        pos += 32;
        out[pos..pos + 32].copy_from_slice(&self.domain_tag);
        Ok(out)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, TxMvpReceiptCommitError> {
        if bytes.len() != TX_MVP_RECEIPT_COMMIT_BYTES {
            return Err(TxMvpReceiptCommitError::InvalidLength);
        }
        let mut pos = 0;
        let version = u32::from_le_bytes(bytes[pos..pos + 4].try_into().unwrap());
        pos += 4;
        let epoch = u64::from_le_bytes(bytes[pos..pos + 8].try_into().unwrap());
        pos += 8;
        let mut nonce = [0u8; 32];
        nonce.copy_from_slice(&bytes[pos..pos + 32]);
        pos += 32;
        let mut payload_commitment = [0u8; 32];
        payload_commitment.copy_from_slice(&bytes[pos..pos + 32]);
        pos += 32;
        let mut disclosure_key_commitment = [0u8; 32];
        disclosure_key_commitment.copy_from_slice(&bytes[pos..pos + 32]);
        pos += 32;
        let mut domain_tag = [0u8; 32];
        domain_tag.copy_from_slice(&bytes[pos..pos + 32]);

        let tx = Self { version, epoch, nonce, payload_commitment, disclosure_key_commitment, domain_tag };
        tx.validate()?;
        Ok(tx)
    }

    pub fn tx_commitment(&self) -> Result<[u8; 32], TxMvpReceiptCommitError> {
        let bytes = self.encode()?;
        let mut hasher = Sha3_256::new();
        hasher.update(b"QASH-MVP-TX-COMMITMENT\0");
        hasher.update(bytes);
        Ok(hasher.finalize().into())
    }

    pub fn nonce_commitment(&self) -> Result<[u8; 32], TxMvpReceiptCommitError> {
        self.validate()?;
        let mut hasher = Sha3_256::new();
        hasher.update(b"QASH-MVP-NONCE-COMMITMENT\0");
        hasher.update(self.epoch.to_le_bytes());
        hasher.update(self.nonce);
        Ok(hasher.finalize().into())
    }

    pub fn public_export(&self) -> Result<TxMvpReceiptCommitPublicExport, TxMvpReceiptCommitError> {
        Ok(TxMvpReceiptCommitPublicExport {
            version: self.version,
            epoch: self.epoch,
            tx_commitment: self.tx_commitment()?,
            nonce_commitment: self.nonce_commitment()?,
            payload_commitment: self.payload_commitment,
            disclosure_key_commitment: self.disclosure_key_commitment,
        })
    }
}

impl TxMvpReceiptCommitPublicExport {
    pub fn encode(&self) -> [u8; TX_MVP_PUBLIC_EXPORT_BYTES] {
        let mut out = [0u8; TX_MVP_PUBLIC_EXPORT_BYTES];
        let mut pos = 0;
        out[pos..pos + 4].copy_from_slice(&self.version.to_le_bytes());
        pos += 4;
        out[pos..pos + 8].copy_from_slice(&self.epoch.to_le_bytes());
        pos += 8;
        out[pos..pos + 32].copy_from_slice(&self.tx_commitment);
        pos += 32;
        out[pos..pos + 32].copy_from_slice(&self.nonce_commitment);
        pos += 32;
        out[pos..pos + 32].copy_from_slice(&self.payload_commitment);
        pos += 32;
        out[pos..pos + 32].copy_from_slice(&self.disclosure_key_commitment);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_bytes(label: &[u8]) -> [u8; 32] {
        let mut hasher = Sha3_256::new();
        hasher.update(label);
        let digest = hasher.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&digest);
        out
    }

    fn sample_tx() -> TxMvpReceiptCommit {
        TxMvpReceiptCommit::new(
            7,
            test_bytes(b"sample-nonce"),
            test_bytes(b"sample-payload-commitment"),
            test_bytes(b"sample-disclosure-key-commitment"),
        )
    }

    #[test]
    fn fixed_size_encoding_roundtrips() {
        let tx = sample_tx();
        let encoded = tx.encode().unwrap();
        assert_eq!(encoded.len(), TX_MVP_RECEIPT_COMMIT_BYTES);
        assert_eq!(TxMvpReceiptCommit::decode(&encoded).unwrap(), tx);
    }

    #[test]
    fn invalid_domain_tag_is_rejected() {
        let mut tx = sample_tx();
        tx.domain_tag = test_bytes(b"invalid-domain-tag");
        assert_eq!(tx.validate(), Err(TxMvpReceiptCommitError::InvalidDomainTag));
    }

    #[test]
    fn invalid_version_is_rejected() {
        let mut tx = sample_tx();
        tx.version = TX_MVP_RECEIPT_COMMIT_VERSION + 1;
        assert_eq!(tx.validate(), Err(TxMvpReceiptCommitError::InvalidVersion));
    }

    #[test]
    fn duplicate_epoch_nonce_is_rejected() {
        let tx = sample_tx();
        let prior = [tx];
        assert_eq!(tx.validate_epoch_nonce_unused(prior.iter()), Err(TxMvpReceiptCommitError::DuplicateEpochNonce));
        let different_epoch = TxMvpReceiptCommit::new(
            8,
            test_bytes(b"sample-nonce"),
            test_bytes(b"sample-payload-commitment"),
            test_bytes(b"sample-disclosure-key-commitment"),
        );
        assert_eq!(different_epoch.validate_epoch_nonce_unused(prior.iter()), Ok(()));
    }

    #[test]
    fn public_export_excludes_raw_nonce_and_domain_tag() {
        let tx = sample_tx();
        let export = tx.public_export().unwrap();
        let encoded = export.encode();
        assert_eq!(encoded.len(), TX_MVP_PUBLIC_EXPORT_BYTES);
        assert!(!encoded.windows(32).any(|window| window == tx.nonce));
        assert!(!encoded.windows(32).any(|window| window == TX_MVP_RECEIPT_COMMIT_DOMAIN_TAG));
        assert!(encoded.windows(32).any(|window| window == tx.payload_commitment));
        assert!(encoded.windows(32).any(|window| window == tx.disclosure_key_commitment));
    }
}
