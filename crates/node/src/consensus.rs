//! Consensus — **Proof of Entropy (PoE)**.
//!
//! No mining. No staking. No wasted energy. Each round, exactly one Probe in the
//! validator set is chosen to propose the next block, and the choice is derived
//! deterministically from the round's **public randomness beacon** (see
//! `entropa_core::beacon`) — so every honest node computes the same proposer without
//! coordination, and no validator (however rich or powerful) can bias who gets
//! selected.
//!
//! This is a stub VRF (hash the beacon). Production replaces `select_proposer`'s inner
//! draw with a verifiable random function whose proof is committed on-chain.

/// Human-readable name of Entropa's consensus mechanism.
pub const NAME: &str = "Proof of Entropy (PoE)";

/// A validator's public identity in the set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Validator {
    /// Probe fingerprint, e.g. `PROBE-1A2B3C4D`.
    pub id: String,
    /// Hex ML-DSA verifying key.
    pub pubkey_hex: String,
}

impl Validator {
    pub fn new(id: impl Into<String>, pubkey_hex: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            pubkey_hex: pubkey_hex.into(),
        }
    }
}

/// Deterministically select the proposer index for a round from the cosmic `beacon`.
/// Returns `None` if the validator set is empty.
pub fn select_proposer(validators: &[Validator], beacon: &str) -> Option<usize> {
    if validators.is_empty() {
        return None;
    }
    let digest = blake3::hash(beacon.as_bytes());
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&digest.as_bytes()[..8]);
    let draw = u64::from_be_bytes(buf);
    Some((draw % validators.len() as u64) as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_set_selects_nobody() {
        assert_eq!(select_proposer(&[], "COSMIC-abc"), None);
    }

    #[test]
    fn selection_is_deterministic() {
        let vs = vec![
            Validator::new("PROBE-A", "aa"),
            Validator::new("PROBE-B", "bb"),
            Validator::new("PROBE-C", "cc"),
        ];
        let a = select_proposer(&vs, "COSMIC-round-7");
        let b = select_proposer(&vs, "COSMIC-round-7");
        assert_eq!(a, b);
        assert!(a.unwrap() < vs.len());
    }
}
