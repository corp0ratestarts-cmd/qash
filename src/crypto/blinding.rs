// Domain B: epoch blinding key derivation.
//
// Key derivation uses entropy (epoch_seed) and therefore lives in Domain B.
// The blinding operations themselves (blind_cascade_input, derive_dilithium_blinding_scalar,
// split_chunk_key) are in qash_consensus::blinding (Domain A — deterministic given the key).

pub use qash_consensus::blinding::{
    BlindingMode, blind_cascade_input,
    derive_dilithium_blinding_scalar,
    split_chunk_key, reconstruct_chunk_key,
};

use qash_consensus::cascade::h_cascade_derive;

/// Derive the epoch blinding key from the epoch's cascade root, epoch index,
/// and entropy seed.
///
/// This is Domain B because `entropy_seed` flows from the PAL layer (nondeterministic
/// entropy source).  The derived key is then passed into Domain A blinding operations.
///
/// epoch_blinding_key = H_cascade_derive(cascade_root, epoch, entropy_seed)
///
/// The result is a 64-byte key suitable for use with all BlindingMode::EpochBoundPRF
/// operations (blind_cascade_input, derive_dilithium_blinding_scalar, split_chunk_key).
pub fn derive_epoch_blinding_key(
    cascade_root: &[u8; 64],
    epoch: u64,
    entropy_seed: &[u8; 32],
) -> [u8; 64] {
    h_cascade_derive(cascade_root, epoch, entropy_seed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_key_is_deterministic() {
        let root  = qash_consensus::cascade::h_cascade(b"test_root");
        let seed  = [0x42u8; 32];
        assert_eq!(
            derive_epoch_blinding_key(&root, 5, &seed),
            derive_epoch_blinding_key(&root, 5, &seed),
        );
    }

    #[test]
    fn epoch_key_differs_across_epochs() {
        let root = qash_consensus::cascade::h_cascade(b"root");
        let seed = [0xABu8; 32];
        assert_ne!(
            derive_epoch_blinding_key(&root, 0, &seed),
            derive_epoch_blinding_key(&root, 1, &seed),
        );
    }

    #[test]
    fn blinding_roundtrip_with_derived_key() {
        let root  = qash_consensus::cascade::h_cascade(b"genesis");
        let seed  = [0x01u8; 32];
        let key   = derive_epoch_blinding_key(&root, 42, &seed);
        let input = b"validator_id_epoch_seed_concat";

        let blinded   = blind_cascade_input(BlindingMode::EpochBoundPRF, &key, input);
        let unblinded = blind_cascade_input(BlindingMode::None, &[], input);
        assert_ne!(blinded, unblinded, "blinded must differ from unblinded");
    }
}
