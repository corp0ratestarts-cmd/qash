#![cfg(feature = "std")]

use qash_pal::smartcard::{
    InMemoryKeyStore, KeyStore, SignRequest, SmartcardError, TokenDescriptor,
};

fn descriptor(provider: &'static str, slot_id: u64, label: &str, serial: &str) -> TokenDescriptor {
    TokenDescriptor {
        provider,
        slot_id,
        label: label.to_string(),
        serial: serial.to_string(),
        mechanisms: vec!["CKM_ECDSA"],
    }
}

#[test]
fn token_descriptor_round_trip() {
    let store = InMemoryKeyStore {
        descriptor: TokenDescriptor {
            provider: "mock-pkcs11",
            slot_id: 7,
            label: "validator-1".to_string(),
            serial: "abc123".to_string(),
            mechanisms: vec!["CKM_ECDSA", "CKM_RSA_PKCS"],
        },
        key_label: "validator-signing".to_string(),
        signing_key: [3u8; 32],
    };

    let descriptor = store.token_descriptor().expect("descriptor should resolve");
    assert_eq!(descriptor.provider, "mock-pkcs11");
    assert_eq!(descriptor.slot_id, 7);
}

#[test]
fn sign_rejects_unknown_key() {
    let store = InMemoryKeyStore {
        descriptor: descriptor("mock-pkcs11", 1, "validator-2", "xyz789"),
        key_label: "expected".to_string(),
        signing_key: [4u8; 32],
    };

    let err = store
        .sign(&SignRequest {
            key_label: "wrong".to_string(),
            payload: vec![1, 2, 3],
        })
        .expect_err("wrong key should fail");

    assert!(matches!(err, SmartcardError::KeyNotFound));
}

#[test]
fn sign_rejects_empty_payload() {
    let store = InMemoryKeyStore {
        descriptor: descriptor("mock-pkcs11", 1, "validator-2", "xyz789"),
        key_label: "expected".to_string(),
        signing_key: [4u8; 32],
    };

    let err = store
        .sign(&SignRequest {
            key_label: "expected".to_string(),
            payload: Vec::new(),
        })
        .expect_err("empty payload should fail");

    assert!(matches!(err, SmartcardError::InvalidInput(_)));
}

#[test]
fn in_memory_signature_is_deterministic_fixed_size_and_redacted() {
    let store = InMemoryKeyStore {
        descriptor: descriptor("mock-pkcs11", 1, "validator-2", "xyz789"),
        key_label: "expected".to_string(),
        signing_key: [4u8; 32],
    };
    let request = SignRequest {
        key_label: "expected".to_string(),
        payload: b"private payload bytes".to_vec(),
    };

    let first = store.sign(&request).expect("signing succeeds");
    let second = store.sign(&request).expect("signing succeeds");

    assert_eq!(first, second);
    assert_eq!(first.len(), 32);
    assert!(!first
        .windows(request.payload.len())
        .any(|window| window == request.payload.as_slice()));
    assert!(!first
        .windows(request.key_label.len())
        .any(|window| window == request.key_label.as_bytes()));
}

#[test]
fn in_memory_signature_binds_payload_key_and_token() {
    let store = InMemoryKeyStore {
        descriptor: descriptor("mock-pkcs11", 1, "validator-2", "xyz789"),
        key_label: "expected".to_string(),
        signing_key: [4u8; 32],
    };
    let other_token = InMemoryKeyStore {
        descriptor: descriptor("mock-pkcs11", 2, "validator-2", "xyz789"),
        key_label: "expected".to_string(),
        signing_key: [4u8; 32],
    };

    let baseline = store
        .sign(&SignRequest {
            key_label: "expected".to_string(),
            payload: b"payload-a".to_vec(),
        })
        .expect("signing succeeds");
    let changed_payload = store
        .sign(&SignRequest {
            key_label: "expected".to_string(),
            payload: b"payload-b".to_vec(),
        })
        .expect("signing succeeds");
    let changed_token = other_token
        .sign(&SignRequest {
            key_label: "expected".to_string(),
            payload: b"payload-a".to_vec(),
        })
        .expect("signing succeeds");

    assert_ne!(baseline, changed_payload);
    assert_ne!(baseline, changed_token);
}

#[test]
fn in_memory_signature_verification_rejects_tampering() {
    let store = InMemoryKeyStore {
        descriptor: descriptor("mock-pkcs11", 1, "validator-2", "xyz789"),
        key_label: "expected".to_string(),
        signing_key: [4u8; 32],
    };
    let request = SignRequest {
        key_label: "expected".to_string(),
        payload: b"payload-a".to_vec(),
    };
    let mut signature = store.sign(&request).expect("signing succeeds");
    store
        .verify(&request, &signature)
        .expect("signature verifies");

    signature[0] ^= 1;
    assert!(matches!(
        store.verify(&request, &signature),
        Err(SmartcardError::Provider(_))
    ));
}
