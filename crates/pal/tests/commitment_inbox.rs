use qash_pal::commitment_inbox::CommitmentInbox;
use qash_pal::commitment_transport::CommitmentFrame;

fn frame(epoch: u64, marker: u8) -> CommitmentFrame {
    CommitmentFrame {
        epoch,
        state_root: [marker; 32],
        receipt_root: [marker.wrapping_add(1); 32],
        efb_root: [marker.wrapping_add(2); 32],
        evidence_root: [marker.wrapping_add(3); 32],
    }
}

#[test]
fn inbox_drains_reordered_commitments_by_epoch() {
    let mut inbox = CommitmentInbox::<8>::new();
    inbox.ingest(frame(30, 30)).unwrap();
    inbox.ingest(frame(10, 10)).unwrap();
    inbox.ingest(frame(20, 20)).unwrap();

    let epochs: Vec<u64> = inbox.drain_ordered().map(|f| f.epoch).collect();
    assert_eq!(epochs, vec![10, 20, 30]);
}

#[test]
fn inbox_handles_delayed_arrival_in_later_drain() {
    let mut inbox = CommitmentInbox::<8>::new();
    inbox.ingest(frame(1, 1)).unwrap();
    inbox.ingest(frame(3, 3)).unwrap();

    let first: Vec<u64> = inbox.drain_ordered().map(|f| f.epoch).collect();
    assert_eq!(first, vec![1, 3]);

    inbox.ingest(frame(2, 2)).unwrap();
    let second: Vec<u64> = inbox.drain_ordered().map(|f| f.epoch).collect();
    assert_eq!(second, vec![2]);
}

#[test]
fn inbox_drops_duplicate_epoch_and_state_root() {
    let mut inbox = CommitmentInbox::<8>::new();
    inbox.ingest(frame(4, 4)).unwrap();
    inbox.ingest(frame(4, 4)).unwrap();

    let drained: Vec<CommitmentFrame> = inbox.drain_ordered().collect();
    assert_eq!(drained.len(), 1);
    assert_eq!(drained[0].epoch, 4);
}
