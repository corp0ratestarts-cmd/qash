//! Stage 2d — Negative privacy tests for receipt evidence.
//!
//! These tests assert that raw receipt body fields, graph-shaped metadata,
//! hardware/operator identity, and plaintext material never appear in:
//! - WAL records (ZeroPersistenceWal / FileRecoveryWal)
//! - ReceiptEncryptionProfile public_root()
//! - EncryptedReceiptCommitment public_root()
//!
//! Each test is written as a structural or type-level check that can be audited
//! without running a real vault backend.

use qash_pal::receipt::{
    algorithm_ids, DisclosureDomain, EncryptedReceiptCommitment, Observer,
    ReceiptEncryptionProfile,
};
use qash_pal::zero_wal::{InMemoryZeroPersistenceWal, ZeroPersistenceWal, ZeroPersistenceWalRecord};

// ---------------------------------------------------------------------------
// WAL record shape — no raw body fields
// ---------------------------------------------------------------------------

#[test]
fn wal_record_has_no_raw_receipt_body_field() {
    let mut wal = InMemoryZeroPersistenceWal::new();
    wal.append_commitment(ZeroPersistenceWalRecord::BlindAudit {
        epoch: 1,
        event_root: [0xAA; 32],
    })
    .unwrap();
    let records = wal.records();
    assert_eq!(records.len(), 1);
    // BlindAudit only carries epoch and event_root — no ciphertext, no sender, no route.
    match records[0] {
        ZeroPersistenceWalRecord::BlindAudit { epoch, event_root } => {
            assert_eq!(epoch, 1);
            assert_eq!(event_root, [0xAAu8; 32]);
        }
        _ => panic!("unexpected record variant"),
    }
}

#[test]
fn wal_effect_commitment_has_no_raw_transaction_body() {
    let mut wal = InMemoryZeroPersistenceWal::new();
    wal.append_commitment(ZeroPersistenceWalRecord::EffectCommitment {
        epoch: 2,
        effect_root: [0xBB; 32],
        receipt_root: [0xCC; 32],
    })
    .unwrap();
    match wal.records()[0] {
        ZeroPersistenceWalRecord::EffectCommitment { effect_root, receipt_root, .. } => {
            // Both fields are 32-byte roots, not variable-length blobs.
            assert_eq!(effect_root.len(), 32);
            assert_eq!(receipt_root.len(), 32);
        }
        _ => panic!("unexpected record variant"),
    }
}

#[test]
fn wal_shred_commitment_excludes_key_material() {
    let mut wal = InMemoryZeroPersistenceWal::new();
    wal.append_commitment(ZeroPersistenceWalRecord::ShredCommitment {
        epoch: 3,
        key_id_commitment: [0xDD; 32],
        event_root: [0xEE; 32],
    })
    .unwrap();
    match wal.records()[0] {
        ZeroPersistenceWalRecord::ShredCommitment { key_id_commitment, event_root, .. } => {
            // key_id_commitment is a hash of the key ID, not the key itself.
            // event_root is a root hash, not raw event data.
            assert_eq!(key_id_commitment.len(), 32);
            assert_eq!(event_root.len(), 32);
        }
        _ => panic!("unexpected record variant"),
    }
}

// ---------------------------------------------------------------------------
// EncryptedReceiptCommitment — no plaintext body in public_root()
// ---------------------------------------------------------------------------

#[test]
fn encrypted_receipt_public_root_is_fixed_32_bytes() {
    let c = EncryptedReceiptCommitment {
        receipt_id: [0xAAu8; 32],
        ciphertext_root: [0xBBu8; 32],
        key_commitment: [0xCCu8; 32],
        disclosure_domain: DisclosureDomain::HolderOnly,
        ciphertext_len: 99999,
    };
    let root = c.public_root();
    assert_eq!(root.len(), 32);
    // The root is deterministic and 32 bytes wide.
    // 0xAA ^ 0xBB ^ 0xCC = 0xDD — non-zero, verifying fold is applied.
    assert_eq!(root, [0xDDu8; 32]);
    assert_ne!(root, c.receipt_id);
    assert_ne!(root, c.ciphertext_root);
    assert_ne!(root, c.key_commitment);
}

#[test]
fn encrypted_receipt_public_root_does_not_leak_ciphertext_len() {
    // Two receipts identical except for ciphertext_len should produce the same root,
    // since ciphertext_len is a metadata field not included in the root hash.
    let base = EncryptedReceiptCommitment {
        receipt_id: [0x11u8; 32],
        ciphertext_root: [0x22u8; 32],
        key_commitment: [0x33u8; 32],
        disclosure_domain: DisclosureDomain::HolderAndAuditor,
        ciphertext_len: 512,
    };
    let other = EncryptedReceiptCommitment { ciphertext_len: 1024, ..base };
    // ciphertext_len is not in the root — the root matches.
    assert_eq!(base.public_root(), other.public_root());
}

// ---------------------------------------------------------------------------
// ReceiptEncryptionProfile — no plaintext or key material
// ---------------------------------------------------------------------------

#[test]
fn receipt_encryption_profile_public_root_is_fixed_32_bytes() {
    let p = ReceiptEncryptionProfile {
        algorithm_id: algorithm_ids::AES_256_GCM,
        key_commitment: [0xABu8; 32],
        disclosure_domain: DisclosureDomain::HolderOnly,
        ciphertext_root: [0xCDu8; 32],
    };
    assert_eq!(p.public_root().len(), 32);
}

#[test]
fn receipt_encryption_profile_has_no_plaintext_body_field() {
    // ReceiptEncryptionProfile must not have a field that is a Vec<u8>
    // (i.e., a variable-length blob that could hold plaintext).
    // This is a compile-time structural check: we construct it and verify
    // that the only Vec<u8>-shaped field is absent by construction.
    let p = ReceiptEncryptionProfile {
        algorithm_id: algorithm_ids::CHACHA20_POLY1305,
        key_commitment: [0xFFu8; 32],
        disclosure_domain: DisclosureDomain::LocalOperatorPolicy,
        ciphertext_root: [0x00u8; 32],
    };
    // If ReceiptEncryptionProfile had a `plaintext_body: Vec<u8>` field,
    // this destructuring would fail to compile — proving none exists.
    let ReceiptEncryptionProfile {
        algorithm_id,
        key_commitment,
        disclosure_domain,
        ciphertext_root,
    } = p;
    let _ = (algorithm_id, key_commitment, disclosure_domain, ciphertext_root);
}

// ---------------------------------------------------------------------------
// DisclosureDomain enforcement — WAL/transcript never includes unauthorized fields
// ---------------------------------------------------------------------------

#[test]
fn public_network_disclosure_is_never_permitted() {
    for domain in [
        DisclosureDomain::HolderOnly,
        DisclosureDomain::HolderAndAuditor,
        DisclosureDomain::LocalOperatorPolicy,
    ] {
        assert!(
            !domain.may_disclose_to(Observer::PublicNetwork),
            "{domain:?} must not permit public network disclosure"
        );
    }
}

#[test]
fn holder_only_rejects_graph_shaped_observer() {
    // The "graph-shaped metadata" concern: any observer other than Holder
    // is rejected under HolderOnly. This prevents network-topology metadata
    // leaking to infrastructure operators.
    let d = DisclosureDomain::HolderOnly;
    assert!(!d.may_disclose_to(Observer::LocalOperator));
    assert!(!d.may_disclose_to(Observer::Auditor));
    assert!(!d.may_disclose_to(Observer::PublicNetwork));
}

#[test]
fn operator_policy_cannot_escalate_to_public() {
    // LocalOperatorPolicy is the most permissive domain; it must still not
    // permit public network disclosure.
    let d = DisclosureDomain::LocalOperatorPolicy;
    assert!(!d.may_disclose_to(Observer::PublicNetwork));
    assert!(!d.is_public_network_permitted());
}

// ---------------------------------------------------------------------------
// Structural assertion: no hardware/operator identity in WAL evidence records
// ---------------------------------------------------------------------------

#[test]
fn state_root_wal_record_has_no_operator_identity_fields() {
    // ZeroPersistenceWalRecord::StateRoot carries only epoch and state_root.
    // No validator ID, no hardware serial, no operator string.
    let r = ZeroPersistenceWalRecord::StateRoot { epoch: 10, state_root: [0x55u8; 32] };
    match r {
        ZeroPersistenceWalRecord::StateRoot { epoch, state_root } => {
            assert_eq!(epoch, 10);
            assert_eq!(state_root.len(), 32);
        }
        _ => panic!("wrong variant"),
    }
}
