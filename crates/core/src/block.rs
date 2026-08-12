//! Blocks and transactions.
//!
//! A [`Block`] is one point in Entropa's growing constellation. It bundles a set of
//! [`Transaction`]s, the cosmic-entropy `beacon` that seeded this round, the proposing
//! Probe's public identity, a blake3 `hash` over the canonical preimage, and a
//! post-quantum `signature` by the proposer over that hash.

use serde::{Deserialize, Serialize};

/// A single transaction — a signal the network records.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Transaction {
    /// Originating actor (a Probe id, or an external identifier).
    pub from: String,
    /// What kind of signal, e.g. `"transfer"`, `"register"`, `"attest"`.
    pub kind: String,
    /// Canonical payload or content-hash.
    pub payload: String,
}

impl Transaction {
    pub fn new(
        from: impl Into<String>,
        kind: impl Into<String>,
        payload: impl Into<String>,
    ) -> Self {
        Self {
            from: from.into(),
            kind: kind.into(),
            payload: payload.into(),
        }
    }
}

/// One block in the chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub index: u64,
    pub timestamp: u64,
    pub prev_hash: String,
    /// Cosmic entropy beacon value that seeded this round.
    pub beacon: String,
    pub transactions: Vec<Transaction>,
    /// Proposing Probe's fingerprint, e.g. `PROBE-1A2B3C4D`.
    pub proposer_id: String,
    /// Proposing Probe's hex ML-DSA verifying key (used to verify `signature`).
    pub proposer_pubkey: String,
    /// blake3 digest (hex) over the canonical preimage.
    pub hash: String,
    /// Proposer's post-quantum ML-DSA signature (hex) over the raw digest bytes.
    pub signature: String,
}

/// Compute the canonical blake3 digest that a block's `hash` commits to and that the
/// proposer signs. Deterministic and order-sensitive — any change to any field
/// changes the digest.
pub fn block_digest(
    index: u64,
    timestamp: u64,
    prev_hash: &str,
    beacon: &str,
    transactions: &[Transaction],
    proposer_id: &str,
) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&index.to_be_bytes());
    hasher.update(&timestamp.to_be_bytes());
    hasher.update(prev_hash.as_bytes());
    hasher.update(beacon.as_bytes());
    hasher.update(proposer_id.as_bytes());
    // Canonical serialization of the transaction set.
    let tx_bytes = serde_json::to_vec(transactions).expect("transactions serialize");
    hasher.update(&tx_bytes);
    *hasher.finalize().as_bytes()
}
