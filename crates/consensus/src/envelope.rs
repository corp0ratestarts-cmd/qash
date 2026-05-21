//! v1.1 Envelope: wire container for consensus inputs with causal ordering metadata.
//!
//! v1.0 envelopes carry only epoch + validator_id + payload.
//! v1.1 envelopes additionally carry epoch_seed, sort_key, and cascade_health,
//! enabling deterministic causal ordering and cascade health gating.

pub const PROTOCOL_VERSION_V1_0: u32 = 0x1000;
pub const PROTOCOL_VERSION_V1_1: u32 = 0x1100;
pub const PROTOCOL_VERSION_V1_2: u32 = 0x1200;

/// A consensus input envelope. `N` is the fixed payload byte length.
///
/// Domain A constraint: all fields are fixed-width integers or byte arrays.
/// No heap allocation, no floats, no usize in state fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Envelope<const N: usize> {
    /// Protocol version: PROTOCOL_VERSION_V1_0 or PROTOCOL_VERSION_V1_1.
    pub version: u32,
    /// Logical epoch at which this envelope was authored.
    pub epoch: u64,
    /// Authoring validator slot index (u32 per Domain A rules).
    pub validator_id: u32,
    /// Execution shard for v1.2+ envelopes. Zero for v1.0/v1.1 compatibility.
    pub shard_id: u32,
    /// Cascade health level at time of envelope creation (v1.1; 0 in v1.0).
    pub cascade_health: u32,
    /// Epoch entropy seed used to compute sort_key (v1.1; zeroed in v1.0).
    pub epoch_seed: [u8; 32],
    /// Causal sort key: H_domain(CausalOrder, epoch_seed ∥ shard_id ∥ hash(payload)).
    /// Zeroed in v1.0 envelopes; must be recomputed by the receiver for v1.1.
    pub sort_key: [u8; 32],
    /// Raw payload bytes (transaction wire format).
    pub payload: [u8; N],
}

impl<const N: usize> Envelope<N> {
    /// Construct a v1.0 envelope (no causal metadata).
    pub fn new_v1_0(epoch: u64, validator_id: u32, payload: [u8; N]) -> Self {
        Self {
            version: PROTOCOL_VERSION_V1_0,
            epoch,
            validator_id,
            shard_id: 0,
            cascade_health: 0,
            epoch_seed: [0u8; 32],
            sort_key: [0u8; 32],
            payload,
        }
    }

    /// Construct a v1.1 envelope with causal ordering metadata.
    pub fn new_v1_1(
        epoch: u64,
        validator_id: u32,
        cascade_health: u32,
        epoch_seed: [u8; 32],
        sort_key: [u8; 32],
        payload: [u8; N],
    ) -> Self {
        Self {
            version: PROTOCOL_VERSION_V1_1,
            epoch,
            validator_id,
            shard_id: 0,
            cascade_health,
            epoch_seed,
            sort_key,
            payload,
        }
    }

    /// Construct a v1.2 envelope with explicit protocol-level shard binding.
    pub fn new_v1_2(
        epoch: u64,
        validator_id: u32,
        shard_id: u32,
        cascade_health: u32,
        epoch_seed: [u8; 32],
        sort_key: [u8; 32],
        payload: [u8; N],
    ) -> Self {
        Self {
            version: PROTOCOL_VERSION_V1_2,
            epoch,
            validator_id,
            shard_id,
            cascade_health,
            epoch_seed,
            sort_key,
            payload,
        }
    }

    /// Returns true if this envelope carries v1.1 (or later) metadata.
    pub fn is_v1_1(&self) -> bool {
        self.version >= PROTOCOL_VERSION_V1_1
    }

    /// Returns true if this envelope carries explicit sharded execution metadata.
    pub fn is_v1_2(&self) -> bool {
        self.version >= PROTOCOL_VERSION_V1_2
    }

    /// Total ordering key: (epoch, sort_key). Deterministic across all ISAs.
    /// For v1.0 envelopes sort_key is zeroed, so ordering falls back to epoch only.
    pub fn causal_sort_key(&self) -> (u64, [u8; 32]) {
        (self.epoch, self.sort_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_0_envelope_fields() {
        let env = Envelope::<4>::new_v1_0(10, 3, [0xAA, 0xBB, 0xCC, 0xDD]);
        assert_eq!(env.version, PROTOCOL_VERSION_V1_0);
        assert_eq!(env.epoch, 10);
        assert_eq!(env.validator_id, 3);
        assert_eq!(env.shard_id, 0);
        assert_eq!(env.cascade_health, 0);
        assert_eq!(env.epoch_seed, [0u8; 32]);
        assert_eq!(env.sort_key, [0u8; 32]);
        assert!(!env.is_v1_1());
    }

    #[test]
    fn v1_1_envelope_fields() {
        let seed = [1u8; 32];
        let key = [2u8; 32];
        let env = Envelope::<4>::new_v1_1(20, 7, 8, seed, key, [0x01, 0x02, 0x03, 0x04]);
        assert_eq!(env.version, PROTOCOL_VERSION_V1_1);
        assert_eq!(env.epoch, 20);
        assert_eq!(env.validator_id, 7);
        assert_eq!(env.shard_id, 0);
        assert_eq!(env.cascade_health, 8);
        assert_eq!(env.epoch_seed, seed);
        assert_eq!(env.sort_key, key);
        assert!(env.is_v1_1());
    }

    #[test]
    fn causal_sort_key_ordering() {
        let seed = [0u8; 32];
        let key_a = [0u8; 32];
        let mut key_b = [0u8; 32];
        key_b[31] = 1;

        let env_a = Envelope::<0>::new_v1_1(5, 0, 0, seed, key_a, []);
        let env_b = Envelope::<0>::new_v1_1(5, 1, 0, seed, key_b, []);

        assert!(env_a.causal_sort_key() < env_b.causal_sort_key());
    }

    #[test]
    fn version_constants_distinct() {
        let versions = [
            PROTOCOL_VERSION_V1_0,
            PROTOCOL_VERSION_V1_1,
            PROTOCOL_VERSION_V1_2,
        ];
        assert_ne!(versions[0], versions[1]);
        assert!(versions[0] < versions[1]);
        assert!(versions[1] < versions[2]);
    }

    #[test]
    fn v1_2_envelope_fields() {
        let seed = [1u8; 32];
        let key = [2u8; 32];
        let env = Envelope::<4>::new_v1_2(20, 7, 3, 8, seed, key, [1, 2, 3, 4]);
        assert_eq!(env.version, PROTOCOL_VERSION_V1_2);
        assert_eq!(env.shard_id, 3);
        assert!(env.is_v1_1());
        assert!(env.is_v1_2());
    }
}
