// Re-exports from the canonical Domain A implementation in qash-consensus.
//
// The astronomical cascade is Domain A logic (pure, deterministic, proof-eligible).
// The authoritative implementation lives in qash_consensus::cascade so it can
// be used directly in the consensus path.  This module re-exports it for use
// in Domain B tools (genesis hash computation, offline clone, etc.).

pub use qash_consensus::cascade::{
    h_cascade, h_cascade_derive, h_cascade_keyed, DOM_SEP_L1, DOM_SEP_L2, DOM_SEP_L3, DOM_SEP_L4,
    DOM_SEP_L5, DOM_SEP_L6, DOM_SEP_L7,
};
