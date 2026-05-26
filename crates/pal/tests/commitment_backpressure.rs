use qash_pal::commitment_backpressure::{BackpressureDecision, CommitmentBackpressure};
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
fn backpressure_admits_then_throttles_then_rejects_commitment_frames() {
    let mut gate = CommitmentBackpressure::new(2, 3).unwrap();
    let mut inbox = CommitmentInbox::<4>::new();

    for (epoch, marker) in [(1, 1), (2, 2)] {
        assert_eq!(
            gate.observe_commitment().unwrap(),
            BackpressureDecision::Admit
        );
        inbox.ingest(frame(epoch, marker)).unwrap();
    }

    assert_eq!(
        gate.observe_commitment().unwrap(),
        BackpressureDecision::Throttle
    );
    inbox.ingest(frame(3, 3)).unwrap();

    assert_eq!(
        gate.observe_commitment().unwrap(),
        BackpressureDecision::Reject
    );
    assert_eq!(inbox.len(), 3);

    let epochs: Vec<u64> = inbox.drain_ordered().map(|f| f.epoch).collect();
    assert_eq!(epochs, vec![1, 2, 3]);
}

#[test]
fn backpressure_window_reset_is_counter_only() {
    let mut gate = CommitmentBackpressure::new(1, 1).unwrap();
    assert_eq!(
        gate.observe_commitment().unwrap(),
        BackpressureDecision::Admit
    );
    assert_eq!(
        gate.observe_commitment().unwrap(),
        BackpressureDecision::Reject
    );
    gate.reset_window();
    assert_eq!(gate.admitted_in_window(), 0);
    assert_eq!(
        gate.observe_commitment().unwrap(),
        BackpressureDecision::Admit
    );
}
