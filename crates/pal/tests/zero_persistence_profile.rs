use qash_pal::admission::{process_envelope, EphemeralEnvelope};
use qash_pal::zero_wal::{InMemoryZeroPersistenceWal, ZeroPersistenceWal, ZeroPersistenceWalRecord};

fn valid_envelope(epoch: u64) -> [u8; 80] {
    let mut bytes = [0u8; 80];
    bytes[0] = 1;
    bytes[8..16].copy_from_slice(&epoch.to_le_bytes());
    bytes[16..48].copy_from_slice(&[21u8; 32]);
    bytes[48..80].copy_from_slice(&[34u8; 32]);
    bytes
}

#[test]
fn production_profile_uses_commitment_only_path() {
    let envelope = valid_envelope(88);
    let slot = EphemeralEnvelope::<128>::new(&envelope).unwrap();
    let effect = process_envelope(slot).unwrap();
    let mut wal = InMemoryZeroPersistenceWal::new();
    wal.append_commitment(ZeroPersistenceWalRecord::from(effect)).unwrap();

    assert_eq!(wal.records().len(), 1);
    assert_eq!(
        wal.records()[0],
        ZeroPersistenceWalRecord::EffectCommitment {
            epoch: 88,
            effect_root: [21u8; 32],
            receipt_root: [34u8; 32],
        }
    );
}
