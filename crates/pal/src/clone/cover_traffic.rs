// Cover traffic for clone relay timing obfuscation.
//
// GENESIS_CONSTANTS.toml [clone_protocol]: cover_traffic = true
//
// Purpose: relay nodes emit constant-rate dummy chunks so that traffic volume
// and inter-chunk timing cannot be used to infer session activity.  Dummy
// chunks are indistinguishable from real chunks on the wire; receivers discard
// them after verifying the dummy domain tag.
//
// Design:
//   - A cover-traffic scheduler tracks the last emission instant and emits a
//     dummy chunk whenever the elapsed time since the previous emission exceeds
//     the target interval.
//   - The dummy chunk payload is a deterministic PRNG output seeded from the
//     epoch seed and a per-relay nonce so that different relays emit distinct
//     byte sequences (prevents correlation by payload equality).
//   - Domain B only — no Domain A state influence.
//
// Wire identification:
//   Dummy chunk payload prefix: DUMMY_MAGIC (8 bytes).
//   Receivers MUST check this prefix and silently drop dummy chunks before
//   cascade proof verification.

use sha3::{Digest, Sha3_256};

/// Prefix embedded in every dummy chunk payload.
pub const DUMMY_MAGIC: &[u8; 8] = b"QASHDMY\0";

/// Default inter-emission interval: one per epoch (500 ms).
pub const DEFAULT_INTERVAL_MS: u64 = 500;

/// Size of a dummy chunk payload including the DUMMY_MAGIC prefix.
pub const DUMMY_PAYLOAD_BYTES: usize = 64;

const DUMMY_DOMAIN: &[u8] = b"QASH/clone/cover-traffic/v1\0";

/// Generates a deterministic dummy chunk payload for `(epoch, seq, relay_nonce)`.
///
/// Output is DUMMY_MAGIC (8 bytes) followed by 56 bytes of pseudorandom filler
/// derived from SHA3-256(DUMMY_DOMAIN || epoch_le8 || seq_le8 || relay_nonce).
/// The filler makes the payload statistically indistinguishable from real data.
pub fn make_dummy_payload(epoch: u64, seq: u64, relay_nonce: &[u8; 32]) -> [u8; DUMMY_PAYLOAD_BYTES] {
    let mut h = Sha3_256::new();
    h.update(DUMMY_DOMAIN);
    h.update(epoch.to_le_bytes());
    h.update(seq.to_le_bytes());
    h.update(relay_nonce);
    let digest: [u8; 32] = h.finalize().into();

    let mut out = [0u8; DUMMY_PAYLOAD_BYTES];
    out[..8].copy_from_slice(DUMMY_MAGIC);
    // Fill remaining 56 bytes: first 32 from digest, next 24 from a second hash.
    out[8..40].copy_from_slice(&digest);
    let mut h2 = Sha3_256::new();
    h2.update(DUMMY_DOMAIN);
    h2.update(b"ext\0");
    h2.update(&digest);
    let ext: [u8; 32] = h2.finalize().into();
    out[40..64].copy_from_slice(&ext[..24]);
    out
}

/// Returns `true` if `payload` is a cover-traffic dummy chunk.
///
/// Real chunk consumers MUST call this before cascade proof verification and
/// drop the chunk if it returns `true`.
pub fn is_dummy_payload(payload: &[u8]) -> bool {
    payload.len() >= 8 && &payload[..8] == DUMMY_MAGIC
}

/// Constant-rate cover-traffic scheduler.
///
/// Tracks elapsed time (in milliseconds, caller-supplied) and signals when a
/// dummy chunk should be emitted to maintain the target emission rate.
pub struct CoverTrafficScheduler {
    interval_ms: u64,
    last_emission_ms: u64,
    seq: u64,
}

impl CoverTrafficScheduler {
    /// Create a scheduler with the given emission interval.
    pub fn new(interval_ms: u64) -> Self {
        Self { interval_ms, last_emission_ms: 0, seq: 0 }
    }

    /// Create a scheduler at the genesis-canonical interval (one per epoch).
    pub fn default_rate() -> Self {
        Self::new(DEFAULT_INTERVAL_MS)
    }

    /// Advance the clock to `now_ms`.  Returns the number of dummy chunks that
    /// should be emitted to catch up to the target rate (normally 0 or 1).
    pub fn tick(&mut self, now_ms: u64) -> u32 {
        if now_ms < self.last_emission_ms {
            return 0;
        }
        let elapsed = now_ms - self.last_emission_ms;
        let due = (elapsed / self.interval_ms) as u32;
        if due > 0 {
            self.last_emission_ms += u64::from(due) * self.interval_ms;
        }
        due
    }

    /// Consume one pending emission slot and return the next dummy payload.
    ///
    /// `epoch` and `relay_nonce` are forwarded to `make_dummy_payload`.
    pub fn next_dummy(&mut self, epoch: u64, relay_nonce: &[u8; 32]) -> [u8; DUMMY_PAYLOAD_BYTES] {
        let payload = make_dummy_payload(epoch, self.seq, relay_nonce);
        self.seq += 1;
        payload
    }

    /// Current emission sequence counter.
    pub fn seq(&self) -> u64 {
        self.seq
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dummy_payload_has_magic_prefix() {
        let p = make_dummy_payload(0, 0, &[0u8; 32]);
        assert_eq!(&p[..8], DUMMY_MAGIC);
        assert_eq!(p.len(), DUMMY_PAYLOAD_BYTES);
    }

    #[test]
    fn is_dummy_correctly_classifies() {
        let p = make_dummy_payload(1, 2, &[0xABu8; 32]);
        assert!(is_dummy_payload(&p));
        assert!(!is_dummy_payload(b"real-chunk-data"));
        assert!(!is_dummy_payload(b""));
    }

    #[test]
    fn dummy_payloads_are_distinct_across_epochs() {
        let nonce = [0u8; 32];
        let a = make_dummy_payload(0, 0, &nonce);
        let b = make_dummy_payload(1, 0, &nonce);
        assert_ne!(a, b);
    }

    #[test]
    fn dummy_payloads_are_distinct_across_seqs() {
        let nonce = [0u8; 32];
        let a = make_dummy_payload(0, 0, &nonce);
        let b = make_dummy_payload(0, 1, &nonce);
        assert_ne!(a, b);
    }

    #[test]
    fn dummy_payloads_are_distinct_across_nonces() {
        let a = make_dummy_payload(0, 0, &[0u8; 32]);
        let b = make_dummy_payload(0, 0, &[1u8; 32]);
        assert_ne!(a, b);
    }

    #[test]
    fn scheduler_emits_one_per_interval() {
        let mut sched = CoverTrafficScheduler::new(500);
        assert_eq!(sched.tick(0), 0);
        assert_eq!(sched.tick(499), 0);
        assert_eq!(sched.tick(500), 1);
        assert_eq!(sched.tick(999), 0);
        assert_eq!(sched.tick(1000), 1);
    }

    #[test]
    fn scheduler_catches_up_on_gap() {
        let mut sched = CoverTrafficScheduler::new(500);
        // Jump forward 3 intervals at once.
        assert_eq!(sched.tick(1500), 3);
        // No more due immediately.
        assert_eq!(sched.tick(1500), 0);
    }

    #[test]
    fn next_dummy_increments_seq() {
        let mut sched = CoverTrafficScheduler::default_rate();
        let nonce = [7u8; 32];
        let p0 = sched.next_dummy(1, &nonce);
        let p1 = sched.next_dummy(1, &nonce);
        assert_ne!(p0, p1);
        assert_eq!(sched.seq(), 2);
    }
}
