use core::fmt::{Debug, Display};

use qash_pal::admission::{AdmissionError, EphemeralEnvelope};
use serde::Serialize;
use static_assertions::{assert_not_impl_any, assert_impl_all};

assert_not_impl_any!(EphemeralEnvelope<128>: Clone, Debug, Display, Send, Sync, Serialize);
assert_impl_all!(AdmissionError: Debug, Copy, Clone, Eq, PartialEq);

#[test]
fn admission_error_debug_is_redacted() {
    let bytes = [0u8; 80];
    let slot = EphemeralEnvelope::<128>::new(&bytes).unwrap();
    let err = qash_pal::admission::process_envelope(slot).unwrap_err();
    let rendered = format!("{err:?}");

    assert!(!rendered.contains("000000"));
    assert!(!rendered.contains("payload"));
    assert!(!rendered.contains("raw"));
    assert!(rendered.contains("SchemaInvalid"));
}
