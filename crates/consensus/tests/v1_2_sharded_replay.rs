/// v1.2 sharded replay corpus.
///
/// The gate pins every epoch's state root, aggregate receipt root, and EFB root
/// for a deterministic two-shard replay sequence. CI should run this on every
/// authorized ISA alongside the v1.1 corpus.
use qash_consensus::envelope::PROTOCOL_VERSION_V1_2;
use qash_consensus::hash::{h_domain, DomainTag};
use qash_consensus::lyapunov::{ConvergenceWindow, ValidatorMetrics};
use qash_consensus::sharding::ShardCommitment;
use qash_consensus::transition::{
    advance_epoch_sharded, EpochInput, EpochShardingInput, EpochState, HaltReason, MAX_VALIDATORS,
};

const CORPUS_EPOCHS: u64 = 12;
const PINNED_JSON: &[u8] = include_bytes!("../../../tests/vectors/vectors.v1.2.json");

fn genesis() -> EpochState {
    EpochState {
        epoch: 0,
        halt_reason: HaltReason::None,
        entropy_seed: [0u8; 32],
        validators: [ValidatorMetrics::ZERO; MAX_VALIDATORS],
        validator_count: 4,
        convergence_window: ConvergenceWindow::new(),
        nonces: [0u64; MAX_VALIDATORS],
        validator_ids: [[0u8; 48]; MAX_VALIDATORS],
        cascade_health: 0,
        state_root: [0u8; 32],
        receipt_root: [0u8; 32],
        efb_root: [0u8; 32],
        causal_fingerprint: [0u8; 32],
    }
}

fn input() -> EpochInput {
    let mut input = EpochInput::new(4);
    input.protocol_version = PROTOCOL_VERSION_V1_2;
    input
}

fn commitment_root(step: u64, shard_id: u32, kind: u8) -> [u8; 32] {
    let mut buf = [0u8; 13];
    buf[0..8].copy_from_slice(&step.to_be_bytes());
    buf[8..12].copy_from_slice(&shard_id.to_be_bytes());
    buf[12] = kind;
    h_domain(DomainTag::EpochFinalityBeacon, &buf)
}

fn shards_for_step(step: u64) -> [ShardCommitment; 2] {
    [
        ShardCommitment {
            shard_id: 0,
            state_root: commitment_root(step, 0, 0),
            receipt_root: commitment_root(step, 0, 1),
        },
        ShardCommitment {
            shard_id: 1,
            state_root: commitment_root(step, 1, 0),
            receipt_root: commitment_root(step, 1, 1),
        },
    ]
}

fn zk_batch_root(step: u64) -> [u8; 32] {
    if step % 3 == 2 {
        commitment_root(step, 0, 2)
    } else {
        [0u8; 32]
    }
}

fn run_corpus() -> std::vec::Vec<(u64, [u8; 32], [u8; 32], [u8; 32])> {
    let mut state = genesis();
    let mut out = std::vec::Vec::with_capacity(CORPUS_EPOCHS as usize);

    for step in 0..CORPUS_EPOCHS {
        let shards = shards_for_step(step);
        let sharding = EpochShardingInput {
            shard_commitments: &shards,
            zk_batch_root: zk_batch_root(step),
        };
        let result = advance_epoch_sharded(&mut state, input().as_effect(), &[], &sharding).unwrap();
        assert_eq!(result.public_transcript.efb_root, state.efb_root);
        assert_eq!(result.public_transcript.receipt_root, state.receipt_root);
        out.push((
            state.epoch,
            state.state_root,
            state.receipt_root,
            state.efb_root,
        ));
    }

    out
}

fn hex_to_bytes32(hex: &str) -> [u8; 32] {
    let bytes: std::vec::Vec<u8> = hex
        .as_bytes()
        .chunks(2)
        .map(|c| u8::from_str_radix(std::str::from_utf8(c).unwrap(), 16).unwrap())
        .collect();
    assert_eq!(bytes.len(), 32, "expected 32-byte root, got {}", hex);
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    arr
}

fn bytes_to_hex(b: &[u8; 32]) -> std::string::String {
    b.iter().map(|byte| format!("{:02x}", byte)).collect()
}

fn load_pinned() -> std::vec::Vec<([u8; 32], [u8; 32], [u8; 32])> {
    let s = std::str::from_utf8(PINNED_JSON).expect("valid UTF-8");
    let v: serde_json::Value = serde_json::from_str(s).expect("valid JSON");
    v["epochs"]
        .as_array()
        .expect("epochs array")
        .iter()
        .map(|entry| {
            (
                hex_to_bytes32(entry["state_root"].as_str().expect("state_root string")),
                hex_to_bytes32(entry["receipt_root"].as_str().expect("receipt_root string")),
                hex_to_bytes32(entry["efb_root"].as_str().expect("efb_root string")),
            )
        })
        .collect()
}

#[test]
fn v1_2_sharded_corpus_matches_pinned() {
    let pinned = load_pinned();
    assert_eq!(pinned.len(), CORPUS_EPOCHS as usize);

    let actual = run_corpus();
    for (i, ((epoch, state_root, receipt_root, efb_root), expected)) in
        actual.iter().zip(pinned.iter()).enumerate()
    {
        assert_eq!(
            state_root,
            &expected.0,
            "state_root mismatch at step {} epoch {}\n  got:      {}\n  expected: {}",
            i,
            epoch,
            bytes_to_hex(state_root),
            bytes_to_hex(&expected.0)
        );
        assert_eq!(
            receipt_root,
            &expected.1,
            "receipt_root mismatch at step {} epoch {}\n  got:      {}\n  expected: {}",
            i,
            epoch,
            bytes_to_hex(receipt_root),
            bytes_to_hex(&expected.1)
        );
        assert_eq!(
            efb_root,
            &expected.2,
            "efb_root mismatch at step {} epoch {}\n  got:      {}\n  expected: {}",
            i,
            epoch,
            bytes_to_hex(efb_root),
            bytes_to_hex(&expected.2)
        );
    }
}

#[test]
#[ignore]
fn gen_v1_2_sharded_corpus() {
    let roots = run_corpus();

    println!("{{");
    println!("  \"version\": \"1.2\",");
    println!("  \"description\": \"12-epoch v1.2 sharded replay corpus over two sorted shard commitments. Pins state_root, aggregate receipt_root, and efb_root for cross-ISA replay verification.\",");
    println!("  \"corpus_epochs\": {},", CORPUS_EPOCHS);
    println!("  \"validator_count\": 4,");
    println!("  \"shard_count\": 2,");
    println!("  \"epochs\": [");
    for (i, (epoch, state_root, receipt_root, efb_root)) in roots.iter().enumerate() {
        let comma = if i + 1 < roots.len() { "," } else { "" };
        println!(
            "    {{\"step\": {}, \"epoch\": {}, \"state_root\": \"{}\", \"receipt_root\": \"{}\", \"efb_root\": \"{}\"}}{}",
            i,
            epoch,
            bytes_to_hex(state_root),
            bytes_to_hex(receipt_root),
            bytes_to_hex(efb_root),
            comma,
        );
    }
    println!("  ]");
    println!("}}");
}
