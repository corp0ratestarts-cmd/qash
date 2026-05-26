#![cfg(feature = "std")]

use qash_pal::admission::{process_envelope, EphemeralEnvelope};
use qash_pal::zero_wal::{
    InMemoryZeroPersistenceWal, ZeroPersistenceWal, ZeroPersistenceWalRecord,
};

fn valid_envelope(epoch: u64) -> [u8; 80] {
    let mut bytes = [0u8; 80];
    bytes[0] = 1;
    bytes[8..16].copy_from_slice(&epoch.to_le_bytes());
    bytes[16..48].copy_from_slice(&[11u8; 32]);
    bytes[48..80].copy_from_slice(&[13u8; 32]);
    bytes
}

#[test]
fn admission_to_wal_persists_commitment_only() {
    let envelope = valid_envelope(77);
    let slot = EphemeralEnvelope::<128>::new(&envelope).unwrap();
    let effect = process_envelope(slot).unwrap();

    let mut wal = InMemoryZeroPersistenceWal::new();
    wal.append_commitment(ZeroPersistenceWalRecord::from(effect))
        .unwrap();

    assert_eq!(
        wal.records(),
        &[ZeroPersistenceWalRecord::EffectCommitment {
            epoch: 77,
            effect_root: [11u8; 32],
            receipt_root: [13u8; 32],
        }]
    );
}

#[test]
fn zero_persistence_wal_has_no_payload_record_shape() {
    let variants = [
        "EffectCommitment",
        "StateRoot",
        "BlindAudit",
        "ShredCommitment",
    ];
    for variant in variants {
        assert!(!variant.contains("Raw"));
        assert!(!variant.contains("Payload"));
        assert!(!variant.contains("Tx"));
    }
}
