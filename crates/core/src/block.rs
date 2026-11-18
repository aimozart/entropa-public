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

    /// A deterministic content fingerprint (hex blake3), used as the public
    /// "Attestation Receipt" identifier — the client gets this back at submit time
    /// and can look up the exact signed block it ended up in later via
    /// `GET /api/receipt/{id}`, independent of which block/index it landed at.
    pub fn content_hash(&self) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(self.from.as_bytes());
        hasher.update(self.kind.as_bytes());
        hasher.update(self.payload.as_bytes());
        hex::encode(hasher.finalize().as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_hash_is_deterministic() {
        let tx = Transaction::new("probe-1", "attest", "hello world");
        assert_eq!(tx.content_hash(), tx.content_hash());
    }

    #[test]
    fn content_hash_differs_for_different_content() {
        let a = Transaction::new("probe-1", "attest", "hello world");
        let b = Transaction::new("probe-1", "attest", "goodbye world");
        assert_ne!(a.content_hash(), b.content_hash());
    }

    #[test]
    fn content_hash_is_sensitive_to_every_field() {
        let base = Transaction::new("probe-1", "attest", "payload");
        let diff_from = Transaction::new("probe-2", "attest", "payload");
        let diff_kind = Transaction::new("probe-1", "register", "payload");
        assert_ne!(base.content_hash(), diff_from.content_hash());
        assert_ne!(base.content_hash(), diff_kind.content_hash());
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
