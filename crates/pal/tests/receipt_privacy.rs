use qash_pal::receipt::{
    DisclosureDomain, EncryptedReceiptCommitment, ReceiptVault, ShredCommitment,
};
use qash_pal::zero_wal::{InMemoryZeroPersistenceWal, ZeroPersistenceWal, ZeroPersistenceWalRecord};

#[derive(Default)]
struct MemoryReceiptVault {
    commitments: Vec<EncryptedReceiptCommitment>,
    shreds: Vec<ShredCommitment>,
}

impl ReceiptVault for MemoryReceiptVault {
    type Error = core::convert::Infallible;

    fn store_commitment(
        &mut self,
        commitment: EncryptedReceiptCommitment,
    ) -> Result<(), Self::Error> {
        self.commitments.push(commitment);
        Ok(())
    }

    fn shred_key(
        &mut self,
        key_id_commitment: [u8; 32],
        epoch: u64,
        event_root: [u8; 32],
    ) -> Result<ShredCommitment, Self::Error> {
        let shred = ShredCommitment {
            key_id_commitment,
            epoch,
            event_root,
        };
        self.shreds.push(shred);
        Ok(shred)
    }
}

#[test]
fn receipt_vault_exposes_commitments_and_shreds_only() {
    let receipt = EncryptedReceiptCommitment {
        receipt_id: [1u8; 32],
        ciphertext_root: [2u8; 32],
        key_commitment: [4u8; 32],
        disclosure_domain: DisclosureDomain::HolderAndAuditor,
        ciphertext_len: 512,
    };

    let mut vault = MemoryReceiptVault::default();
    vault.store_commitment(receipt).unwrap();
    let shred = vault.shred_key([8u8; 32], 12, [9u8; 32]).unwrap();

    assert_eq!(vault.commitments[0].public_root(), [7u8; 32]);
    assert_eq!(shred.epoch, 12);
    assert_eq!(shred.event_root, [9u8; 32]);
}

#[test]
fn receipt_roots_can_be_persisted_without_receipt_body() {
    let receipt = EncryptedReceiptCommitment {
        receipt_id: [3u8; 32],
        ciphertext_root: [5u8; 32],
        key_commitment: [6u8; 32],
        disclosure_domain: DisclosureDomain::HolderOnly,
        ciphertext_len: 256,
    };

    let mut wal = InMemoryZeroPersistenceWal::new();
    wal.append_commitment(ZeroPersistenceWalRecord::BlindAudit {
        epoch: 44,
        event_root: receipt.public_root(),
    })
    .unwrap();

    assert_eq!(wal.records().len(), 1);
}
