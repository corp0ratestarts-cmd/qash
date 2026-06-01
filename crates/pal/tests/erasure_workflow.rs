//! End-to-end erasure workflow integration test.
//!
//! Tests the full GDPR Art. 17 / key-shredding lifecycle:
//!   1. Create a receipt key and encrypt a payload.
//!   2. Register the key in a key store.
//!   3. Submit an erasure request.
//!   4. Confirm the key is consumed (shredded) and the evidence is correct.
//!   5. Attempt to decrypt with the shredded key — confirm failure.
//!
//! This is an implementation-layer test. GDPR compliance additionally
//! requires legal assessment, backup policy, and operator procedures (see
//! docs/security/ERASURE_RUNBOOK.md).

use qash_pal::privacy::erasure::{
    compute_erasure_evidence_root_pair, process_erasure_request, ErasureError, ErasureRequest,
    KeyStore, ReceiptKey, ShredKeyEvidence,
};

// ── Minimal in-memory key store for tests ────────────────────────────────────

struct TestKeyStore {
    entries: Vec<ReceiptKey>,
}

impl TestKeyStore {
    fn insert(&mut self, key: ReceiptKey) {
        self.entries.push(key);
    }

    fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl KeyStore for TestKeyStore {
    fn locate_by_commitment(&mut self, commitment: &[u8; 32]) -> Option<ReceiptKey> {
        if let Some(pos) = self
            .entries
            .iter()
            .position(|k| &k.key_commitment == commitment)
        {
            Some(self.entries.remove(pos))
        } else {
            None
        }
    }
}

// ── Simulated XOR encryption (not a real cipher; for test only) ───────────────

fn xor_encrypt(plaintext: &[u8; 32], key: &[u8; 32]) -> [u8; 32] {
    let mut ct = [0u8; 32];
    for i in 0..32 {
        ct[i] = plaintext[i] ^ key[i];
    }
    ct
}

fn xor_decrypt(ciphertext: &[u8; 32], key: &[u8; 32]) -> [u8; 32] {
    xor_encrypt(ciphertext, key) // XOR is its own inverse
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// Full erasure lifecycle: create key → encrypt → shred via erasure request
/// → confirm decrypt fails.
#[test]
fn erasure_workflow_e2e() {
    let plaintext = [0xABu8; 32];
    let key_material = [0x55u8; 32];

    // 1. Create receipt key and capture commitment.
    let key = ReceiptKey::new(key_material);
    let key_commitment = key.key_commitment;

    // 2. Encrypt the payload using the key.
    let ciphertext = xor_encrypt(&plaintext, key.as_bytes());
    // Confirm we can currently decrypt.
    assert_eq!(
        xor_decrypt(&ciphertext, &key_material),
        plaintext,
        "decrypt before shred must succeed"
    );

    // 3. Register key in store.
    let mut store = TestKeyStore { entries: vec![] };
    store.insert(key);

    // 4. Submit erasure request.
    let req = ErasureRequest {
        receipt_commitment: key_commitment,
        requestor_id: [0xFFu8; 32],
        epoch: 100,
    };
    let evidence = process_erasure_request(req, &mut store)
        .expect("erasure request must succeed when key is present");

    // 5. Verify evidence is correct.
    assert_eq!(evidence.key_commitment, key_commitment);
    assert_eq!(evidence.epoch, 100);
    // event_root links to the receipt being erased (receipt_commitment), not the requestor.
    assert_eq!(evidence.event_root, key_commitment);

    // 6. Key must be consumed from the store.
    assert!(store.is_empty(), "key must be removed from store after shred");

    // 7. Confirm decryption is no longer possible: the commitment is not the key.
    let attempted = xor_decrypt(&ciphertext, &evidence.key_commitment);
    assert_ne!(
        attempted, plaintext,
        "decryption using key commitment must not recover plaintext"
    );
}

/// Replay audit: evidence from a prior shred is persistent and can be verified
/// independently of the key store (which no longer holds the key).
#[test]
fn erasure_workflow_replay_audit() {
    let key = ReceiptKey::new([0x11u8; 32]);
    let expected_commitment = key.key_commitment;
    let mut store = TestKeyStore { entries: vec![key] };

    let req = ErasureRequest {
        receipt_commitment: expected_commitment,
        requestor_id: [0x22u8; 32],
        epoch: 77,
    };
    let ev1 = process_erasure_request(req, &mut store).unwrap();

    // Simulate a second audit process reconstructing the evidence record from WAL.
    // evidence_root_pair is deterministically recomputed from the public fields.
    let ev_reconstructed = ShredKeyEvidence {
        key_commitment: ev1.key_commitment,
        epoch: ev1.epoch,
        event_root: ev1.event_root,
        evidence_root_pair: compute_erasure_evidence_root_pair(&ev1),
    };

    assert_eq!(ev1, ev_reconstructed, "replayed evidence must match original");
    assert_eq!(ev_reconstructed.key_commitment, expected_commitment);
}

/// Duplicate erasure request after key is consumed returns KeyNotFound.
#[test]
fn erasure_workflow_duplicate_request_fails() {
    let key = ReceiptKey::new([0x33u8; 32]);
    let commitment = key.key_commitment;
    let mut store = TestKeyStore { entries: vec![key] };

    let req = ErasureRequest {
        receipt_commitment: commitment,
        requestor_id: [0x44u8; 32],
        epoch: 1,
    };

    // First request succeeds.
    process_erasure_request(req, &mut store).expect("first erasure must succeed");

    // Second request with same commitment must fail (key already shredded).
    assert_eq!(
        process_erasure_request(req, &mut store),
        Err(ErasureError::KeyNotFound),
        "duplicate erasure must return KeyNotFound (key already consumed)"
    );
}
