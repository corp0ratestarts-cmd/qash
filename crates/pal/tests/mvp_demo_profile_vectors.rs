use qash_pal::mvp::{
    TxMvpReceiptCommit, TxMvpReceiptCommitPublicExport, TxMvpReceiptCommitError,
    TX_MVP_PUBLIC_EXPORT_BYTES, TX_MVP_RECEIPT_COMMIT_VERSION,
};
use sha3::{Digest, Sha3_256};

#[derive(Clone, Copy)]
enum FixtureKind {
    FirstNonce,
    SecondNonce,
    FirstPayloadCommitment,
    SecondPayloadCommitment,
    DisclosureCommitment,
}

fn fixture_bytes(kind: FixtureKind) -> [u8; 32] {
    core::array::from_fn(|idx| {
        let base = idx as u8;
        match kind {
            FixtureKind::FirstNonce => base.rotate_left(1),
            FixtureKind::SecondNonce => base.rotate_right(1),
            FixtureKind::FirstPayloadCommitment => !base,
            FixtureKind::SecondPayloadCommitment => base.reverse_bits(),
            FixtureKind::DisclosureCommitment => (!base).rotate_left(1),
        }
    })
}

fn public_export_sequence() -> [TxMvpReceiptCommitPublicExport; 2] {
    let first = TxMvpReceiptCommit::new(
        10,
        fixture_bytes(FixtureKind::FirstNonce),
        fixture_bytes(FixtureKind::FirstPayloadCommitment),
        fixture_bytes(FixtureKind::DisclosureCommitment),
    )
    .public_export()
    .expect("valid public export fixture");

    let second = TxMvpReceiptCommit::new(
        11,
        fixture_bytes(FixtureKind::SecondNonce),
        fixture_bytes(FixtureKind::SecondPayloadCommitment),
        fixture_bytes(FixtureKind::DisclosureCommitment),
    )
    .public_export()
    .expect("valid public export fixture");

    [first, second]
}

fn demo_profile_root(records: &[TxMvpReceiptCommitPublicExport]) -> [u8; 32] {
    let mut root = [0u8; 32];
    for record in records {
        let encoded = record.encode();
        let mut hasher = Sha3_256::new();
        hasher.update(b"QASH-MVP-DEMO-PROFILE-ROOT\0");
        hasher.update(root);
        hasher.update((encoded.len() as u64).to_le_bytes());
        hasher.update(encoded);
        root = hasher.finalize().into();
    }
    root
}

#[test]
fn public_exports_decode_and_replay_to_stable_root() {
    let records = public_export_sequence();
    for record in &records {
        let encoded = record.encode();
        assert_eq!(encoded.len(), TX_MVP_PUBLIC_EXPORT_BYTES);
        assert_eq!(TxMvpReceiptCommitPublicExport::decode(&encoded).unwrap(), *record);
    }

    let root_once = demo_profile_root(&records);
    let root_twice = demo_profile_root(&records);
    assert_eq!(root_once, root_twice);
}

#[test]
fn invalid_public_export_shapes_fail_closed() {
    let records = public_export_sequence();
    let mut encoded = records[0].encode();

    encoded[0..4].copy_from_slice(&(TX_MVP_RECEIPT_COMMIT_VERSION + 1).to_le_bytes());
    assert_eq!(
        TxMvpReceiptCommitPublicExport::decode(&encoded),
        Err(TxMvpReceiptCommitError::InvalidVersion)
    );

    let valid = records[0].encode();
    assert_eq!(
        TxMvpReceiptCommitPublicExport::decode(&valid[..TX_MVP_PUBLIC_EXPORT_BYTES - 1]),
        Err(TxMvpReceiptCommitError::InvalidLength)
    );

    let mut extra = valid.to_vec();
    extra.push(0);
    assert_eq!(
        TxMvpReceiptCommitPublicExport::decode(&extra),
        Err(TxMvpReceiptCommitError::InvalidLength)
    );
}

#[test]
fn public_export_replay_is_order_sensitive_until_order_rule_is_defined() {
    let records = public_export_sequence();
    let forward = demo_profile_root(&records);
    let reversed = demo_profile_root(&[records[1], records[0]]);
    assert_ne!(forward, reversed);
}
