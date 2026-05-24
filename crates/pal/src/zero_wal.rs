//! Commitment-only WAL records for zero-persistence production mode.

use crate::admission::ValidatedEffectCommitment;
use crate::receipt::ShredCommitment;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZeroPersistenceWalRecord {
    EffectCommitment {
        epoch: u64,
        effect_root: [u8; 32],
        receipt_root: [u8; 32],
    },
    StateRoot {
        epoch: u64,
        state_root: [u8; 32],
    },
    BlindAudit {
        epoch: u64,
        event_root: [u8; 32],
    },
    ShredCommitment {
        epoch: u64,
        key_id_commitment: [u8; 32],
        event_root: [u8; 32],
    },
}

impl From<ValidatedEffectCommitment> for ZeroPersistenceWalRecord {
    fn from(value: ValidatedEffectCommitment) -> Self {
        Self::EffectCommitment {
            epoch: value.epoch,
            effect_root: value.effect_root,
            receipt_root: value.receipt_root,
        }
    }
}

impl From<ShredCommitment> for ZeroPersistenceWalRecord {
    fn from(value: ShredCommitment) -> Self {
        Self::ShredCommitment {
            epoch: value.epoch,
            key_id_commitment: value.key_id_commitment,
            event_root: value.event_root,
        }
    }
}

pub trait ZeroPersistenceWal {
    type Error;
    fn append_commitment(&mut self, record: ZeroPersistenceWalRecord) -> Result<(), Self::Error>;
}

#[cfg(feature = "std")]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct InMemoryZeroPersistenceWal {
    records: Vec<ZeroPersistenceWalRecord>,
}

#[cfg(feature = "std")]
impl InMemoryZeroPersistenceWal {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn records(&self) -> &[ZeroPersistenceWalRecord] {
        &self.records
    }
}

#[cfg(feature = "std")]
impl ZeroPersistenceWal for InMemoryZeroPersistenceWal {
    type Error = core::convert::Infallible;

    fn append_commitment(&mut self, record: ZeroPersistenceWalRecord) -> Result<(), Self::Error> {
        self.records.push(record);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validated_effect_maps_to_commitment_record() {
        let effect = ValidatedEffectCommitment {
            epoch: 7,
            effect_root: [1u8; 32],
            receipt_root: [2u8; 32],
        };
        assert_eq!(
            ZeroPersistenceWalRecord::from(effect),
            ZeroPersistenceWalRecord::EffectCommitment {
                epoch: 7,
                effect_root: [1u8; 32],
                receipt_root: [2u8; 32],
            }
        );
    }

    #[test]
    fn shred_commitment_maps_to_dedicated_record() {
        let shred = ShredCommitment {
            epoch: 11,
            key_id_commitment: [8u8; 32],
            event_root: [9u8; 32],
        };
        assert_eq!(
            ZeroPersistenceWalRecord::from(shred),
            ZeroPersistenceWalRecord::ShredCommitment {
                epoch: 11,
                key_id_commitment: [8u8; 32],
                event_root: [9u8; 32],
            }
        );
    }

    #[cfg(feature = "std")]
    #[test]
    fn in_memory_wal_persists_commitments_only() {
        let mut wal = InMemoryZeroPersistenceWal::new();
        wal.append_commitment(ZeroPersistenceWalRecord::EffectCommitment {
            epoch: 1,
            effect_root: [3u8; 32],
            receipt_root: [4u8; 32],
        })
        .unwrap();
        assert_eq!(wal.records().len(), 1);
    }
}
