#![cfg(all(feature = "std", feature = "mock_signatures"))]

use qash_pal::smartcard::{
    InMemoryKeyStore, KeyStore, SignRequest, SmartcardError, TokenDescriptor,
};

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
    };

    let descriptor = store.token_descriptor().expect("descriptor should resolve");
    assert_eq!(descriptor.provider, "mock-pkcs11");
    assert_eq!(descriptor.slot_id, 7);
}

#[test]
fn sign_rejects_unknown_key() {
    let store = InMemoryKeyStore {
        descriptor: TokenDescriptor {
            provider: "mock-pkcs11",
            slot_id: 1,
            label: "validator-2".to_string(),
            serial: "xyz789".to_string(),
            mechanisms: vec!["CKM_ECDSA"],
        },
        key_label: "expected".to_string(),
    };

    let err = store
        .sign(&SignRequest {
            key_label: "wrong".to_string(),
            payload: vec![1, 2, 3],
        })
        .expect_err("wrong key should fail");

    assert!(matches!(err, SmartcardError::KeyNotFound));
}
