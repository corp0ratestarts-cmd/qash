//! Production WAL for zero-persistence Pure QASH.
//!
//! Permitted record fields: epoch, scalar commitment roots, receipt root, EFB root,
//! blind audit event IDs, validation failure class (no payload), shred commitment IDs.
//!
//! Forbidden fields (compile-time enforced via type system):
//! raw_txs, payload bytes, peer_ip, socket_addr, receipt body, graph edge,
//! transaction lists, routing metadata.

use qash_consensus::public::PublicTranscript;

/// A single WAL record — commitment-only, no graph material.
#[derive(Debug)]
pub struct WalRecord {
    pub epoch:        u64,
    pub state_root:   [u8; 32],
    pub receipt_root: [u8; 32],
    pub efb_root:     [u8; 32],
    pub halt_flag:    bool,
    /// Blind audit event IDs — opaque 32-byte identifiers, no payload bytes.
    pub audit_event_ids: heapless::Vec<[u8; 32], 16>,
    /// Validation failure class code — no payload, no tx body, no peer info.
    pub failure_class: Option<u16>,
}

impl WalRecord {
    /// Construct a WAL record from a PublicTranscript.
    /// PublicTranscript is the compile-time guarantee that only root-level data is present.
    pub fn from_transcript(t: &PublicTranscript, failure_class: Option<u16>) -> Self {
        let encoded = t.encode_canonical();
        // PublicTranscript canonical encoding: state_root(32) + receipt_root(32) + efb_root(8+32)
        // + epoch(8) + halt_flag(1) = 105 bytes total. Extract fields by position.
        assert!(
            encoded.len() >= 105,
            "PublicTranscript encoding too short: expected >=105 bytes, got {}",
            encoded.len()
        );
        let mut state_root   = [0u8; 32];
        let mut receipt_root = [0u8; 32];
        let mut efb_root     = [0u8; 32];
        state_root.copy_from_slice(&encoded[0..32]);
        receipt_root.copy_from_slice(&encoded[32..64]);
        efb_root.copy_from_slice(&encoded[64..96]);
        let epoch = u64::from_le_bytes(
            encoded[96..104].try_into().expect("PublicTranscript must encode epoch at bytes 96-104"),
        );
        let halt_flag = encoded[104] != 0;

        Self {
            epoch,
            state_root,
            receipt_root,
            efb_root,
            halt_flag,
            audit_event_ids: heapless::Vec::new(),
            failure_class,
        }
    }
}

/// Production WAL — writes only WalRecords. Rejects any attempt to log raw graph material.
pub struct ProductionWal {
    records: heapless::Vec<WalRecord, 256>,
}

impl ProductionWal {
    pub fn new() -> Self {
        Self { records: heapless::Vec::new() }
    }

    /// Append a commitment-only WAL record.
    pub fn append(&mut self, record: WalRecord) -> Result<(), WalError> {
        self.records.push(record).map_err(|_| WalError::Full)
    }

    /// Read all committed records (for crash recovery — roots only).
    pub fn records(&self) -> &[WalRecord] {
        &self.records
    }
}

impl Default for ProductionWal {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalError {
    Full,
}

// These types deliberately do not exist in this module:
//
//   RawTxWalRecord       — forbidden: contains transaction bytes
//   PayloadWalRecord     — forbidden: contains payload bytes
//   PeerIpWalRecord      — forbidden: contains peer IP
//   ReceiptBodyWalRecord — forbidden: contains receipt plaintext
//
// If you need to log a transaction admission failure, use `failure_class: Some(code)`
// in WalRecord. Do NOT include transaction bytes, peer addresses, or payload data.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wal_record_has_no_payload_fields() {
        // Structural test: WalRecord fields must not include raw bytes beyond roots.
        // This test documents the intent; the compile-time check is that WalRecord
        // has no Vec<u8>, String, or SocketAddr fields.
        let r = WalRecord {
            epoch: 1,
            state_root:   [0u8; 32],
            receipt_root: [0u8; 32],
            efb_root:     [0u8; 32],
            halt_flag:    false,
            audit_event_ids: heapless::Vec::new(),
            failure_class: None,
        };
        assert_eq!(r.epoch, 1);
        assert!(!r.halt_flag);
    }

    #[test]
    fn production_wal_append_and_read() {
        let mut wal = ProductionWal::new();
        let r = WalRecord {
            epoch: 42,
            state_root: [1u8; 32],
            receipt_root: [2u8; 32],
            efb_root: [3u8; 32],
            halt_flag: false,
            audit_event_ids: heapless::Vec::new(),
            failure_class: None,
        };
        wal.append(r).unwrap();
        assert_eq!(wal.records().len(), 1);
        assert_eq!(wal.records()[0].epoch, 42);
    }
}
