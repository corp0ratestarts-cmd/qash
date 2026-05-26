#![cfg(feature = "std")]

use qash_pal::commitment_transport::{
    CommitmentFrame, CommitmentTransport, InMemoryCommitmentTransport,
};
use qash_pal::recovery_wal::FileRecoveryWal;
use qash_pal::zero_wal::ZeroPersistenceWalRecord;

#[test]
fn commitment_transport_and_recovery_wal_round_trip() {
    let frame = CommitmentFrame {
        epoch: 6,
        state_root: [1u8; 32],
        receipt_root: [2u8; 32],
        efb_root: [3u8; 32],
        evidence_root: [4u8; 32],
    };

    let mut transport = InMemoryCommitmentTransport::new();
    transport.send_commitment(frame).unwrap();
    let received = transport.recv_commitment().unwrap().unwrap();
    assert_eq!(received, frame);

    let mut path = std::env::temp_dir();
    path.push(format!(
        "qash-recovery-{}-{}.wal",
        std::process::id(),
        received.epoch
    ));
    let _ = std::fs::remove_file(&path);

    let wal = FileRecoveryWal::open(&path).unwrap();
    wal.append_synced(ZeroPersistenceWalRecord::StateRoot {
        epoch: received.epoch,
        state_root: received.state_root,
    })
    .unwrap();
    wal.append_synced(ZeroPersistenceWalRecord::BlindAudit {
        epoch: received.epoch,
        event_root: received.evidence_root,
    })
    .unwrap();

    let reopened = FileRecoveryWal::open(&path).unwrap();
    let replayed = reopened.replay().unwrap();
    assert_eq!(replayed.len(), 2);
    assert_eq!(
        replayed[0],
        ZeroPersistenceWalRecord::StateRoot {
            epoch: 6,
            state_root: [1u8; 32],
        }
    );
    assert_eq!(
        replayed[1],
        ZeroPersistenceWalRecord::BlindAudit {
            epoch: 6,
            event_root: [4u8; 32],
        }
    );

    let _ = std::fs::remove_file(&path);
}
