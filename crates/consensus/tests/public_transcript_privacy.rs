use qash_consensus::PublicTranscript;

#[test]
fn public_transcript_is_root_only() {
    let transcript = PublicTranscript {
        state_root: [1u8; 32],
        receipt_root: [2u8; 32],
        efb_root: [3u8; 32],
        epoch: 4,
        halt_flag: false,
    };

    assert_eq!(transcript.state_root, [1u8; 32]);
    assert_eq!(transcript.receipt_root, [2u8; 32]);
    assert_eq!(transcript.efb_root, [3u8; 32]);
    assert_eq!(transcript.epoch, 4);
    assert!(!transcript.halt_flag);
}

#[test]
fn public_transcript_type_name_has_no_graph_or_identity_surface() {
    let type_name = core::any::type_name::<PublicTranscript>();
    for forbidden in [
        "Raw",
        "Tx",
        "Graph",
        "Edge",
        "Peer",
        "Ip",
        "Socket",
        "ReceiptBody",
        "Payload",
        "Aaguid",
        "Serial",
        "Hardware",
        "Operator",
        "ValidatorIdentity",
    ] {
        assert!(
            !type_name.contains(forbidden),
            "forbidden marker {forbidden}"
        );
    }
}
