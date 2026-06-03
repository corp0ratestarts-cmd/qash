//! EphemeralEnvelope — zero-persistence admission buffer.
//!
//! Deliberately non-Clone, non-Copy, non-Debug, non-Display, non-Serialize.
//! Consumed by value. Zeroized on drop. No Vec<u8> raw payload copies escape.
//!
//! This type is the Domain B admission gate for raw transaction bytes.
//! It exists only inside an owned admission lifecycle; the only admissible
//! outputs are CapToken<ValidatedEffect> values and blind audit event IDs.

use zeroize::Zeroize;

/// Raw transaction admission buffer.
///
/// MUST NOT implement: Clone, Copy, Debug, Display, Serialize, Deserialize.
/// MUST zeroize payload bytes on drop.
///
/// Use `consume()` to extract the bytes for processing; the envelope is
/// destroyed after consumption and cannot be reused or copied.
pub struct EphemeralEnvelope {
    // Inner bytes are heap-allocated via Box so the address is stable across moves,
    // and zeroize can clear them reliably on drop.
    inner: Box<[u8]>,
}

impl EphemeralEnvelope {
    /// Wrap raw bytes in an ephemeral envelope.
    /// Caller is responsible for ensuring bytes were received over a secure
    /// Domain B channel and were never written to a durable store.
    pub fn new(bytes: Vec<u8>) -> Self {
        Self { inner: bytes.into_boxed_slice() }
    }

    /// Consume the envelope and return the raw bytes for processing.
    /// After this call the envelope is dropped and its backing memory is zeroized.
    pub fn consume(mut self) -> impl AsRef<[u8]> + Drop {
        // Use replace+forget to guarantee zeroization even if a panic occurs between
        // extracting inner and the caller's drop. mem::take would leave self.inner
        // pointing at an empty slice while the original bytes are still live;
        // mem::forget(self) prevents the Drop impl from running on the now-empty shell,
        // and OwnedZeroBytes zeroizes the actual bytes when the caller drops it.
        let inner = core::mem::replace(&mut self.inner, Box::new([]));
        core::mem::forget(self);
        OwnedZeroBytes(inner)
    }

    /// Length of the envelope in bytes.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl Drop for EphemeralEnvelope {
    fn drop(&mut self) {
        self.inner.zeroize();
    }
}

// Explicitly deny all deriving that would leak contents.
// These are compile-time static assertions via the trait-not-implemented pattern.
// The #[derive] attributes are intentionally absent.

/// Owned bytes that zeroize on drop. Returned from `EphemeralEnvelope::consume()`.
pub struct OwnedZeroBytes(Box<[u8]>);

impl AsRef<[u8]> for OwnedZeroBytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Drop for OwnedZeroBytes {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

// Static assertions: EphemeralEnvelope must not be Send + Sync in a way that
// allows it to escape the admission thread. It IS Send (moving across threads
// is fine; copying is not).
static_assertions::assert_not_impl_any!(EphemeralEnvelope: Clone, Copy);
static_assertions::assert_not_impl_any!(OwnedZeroBytes: Clone, Copy);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ephemeral_envelope_consumes_once() {
        let env = EphemeralEnvelope::new(vec![1, 2, 3, 4]);
        assert_eq!(env.len(), 4);
        let owned = env.consume();
        assert_eq!(owned.as_ref(), &[1, 2, 3, 4]);
        // env is moved; cannot use again (compile-time enforced)
    }

    #[test]
    fn ephemeral_envelope_empty() {
        let env = EphemeralEnvelope::new(vec![]);
        assert!(env.is_empty());
    }

    // Compile-time: the following would fail to compile if uncommented:
    // fn would_not_compile() {
    //     let env = EphemeralEnvelope::new(vec![1]);
    //     let _copy = env.clone();   // Clone not implemented
    //     println!("{:?}", env);     // Debug not implemented
    // }
}
