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

/// Parse `ENTROPA_VALIDATORS`-style config: comma-separated `id:pubkey_hex` pairs,
/// identical across every validator process (each of the 3 needs to know all 3
/// members up front, not just itself). Returns `None` on any malformed entry rather
/// than silently dropping it — a partially wrong validator set is worse than
/// refusing to start.
pub fn parse_validators(spec: &str) -> Option<Vec<Validator>> {
    spec.split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| {
            let (id, pubkey_hex) = entry.split_once(':')?;
            if id.is_empty() || pubkey_hex.is_empty() {
                return None;
            }
            Some(Validator::new(id, pubkey_hex))
        })
        .collect()
}

/// Parse `ENTROPA_PEER_URLS`-style config: comma-separated peer base URLs (this
/// validator's own URL is never included — only the *other* validators to attest
/// with). Blank entries are skipped; an entirely blank/empty spec yields an empty
/// list rather than `None`, since running with zero configured peers is a valid
/// (if degenerate, single-validator) state, not a malformed one.
pub fn parse_peer_urls(spec: &str) -> Vec<String> {
    spec.split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(str::to_string)
        .collect()
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

    // --- Phase 4b: ENTROPA_VALIDATORS / ENTROPA_PEER_URLS parsing ---
    //
    // Each of the 3 validator processes needs the *same* full validator set
    // (id:pubkey_hex for all 3) and its *own* list of peer URLs to attest with —
    // both currently hardcoded to a single entry in main.rs.

    #[test]
    fn parses_a_well_formed_validator_spec() {
        let spec = "PROBE-AAAA:aabbcc,PROBE-BBBB:ddeeff";
        let parsed = parse_validators(spec).expect("well-formed spec parses");
        assert_eq!(
            parsed,
            vec![
                Validator::new("PROBE-AAAA", "aabbcc"),
                Validator::new("PROBE-BBBB", "ddeeff"),
            ]
        );
    }

    #[test]
    fn validator_spec_tolerates_surrounding_whitespace() {
        let spec = " PROBE-AAAA:aabbcc , PROBE-BBBB:ddeeff ";
        let parsed = parse_validators(spec).expect("whitespace is trimmed");
        assert_eq!(
            parsed,
            vec![
                Validator::new("PROBE-AAAA", "aabbcc"),
                Validator::new("PROBE-BBBB", "ddeeff"),
            ]
        );
    }

    #[test]
    fn validator_entry_missing_the_colon_separator_is_rejected() {
        assert_eq!(parse_validators("PROBE-AAAA-no-colon"), None);
    }

    #[test]
    fn validator_entry_with_an_empty_id_or_pubkey_is_rejected() {
        assert_eq!(parse_validators(":aabbcc"), None);
        assert_eq!(parse_validators("PROBE-AAAA:"), None);
    }

    #[test]
    fn one_malformed_entry_invalidates_the_whole_spec() {
        // A partially-wrong validator set silently starting is worse than refusing
        // to start at all - never drop just the bad entry and continue with the rest.
        let spec = "PROBE-AAAA:aabbcc,this-entry-is-bad,PROBE-BBBB:ddeeff";
        assert_eq!(parse_validators(spec), None);
    }

    #[test]
    fn parses_peer_urls_and_trims_whitespace() {
        let spec = " http://peer-a:8080 , http://peer-b:8080 ";
        assert_eq!(
            parse_peer_urls(spec),
            vec![
                "http://peer-a:8080".to_string(),
                "http://peer-b:8080".to_string()
            ]
        );
    }

    #[test]
    fn blank_peer_url_spec_yields_an_empty_list_not_a_blank_entry() {
        assert_eq!(parse_peer_urls(""), Vec::<String>::new());
        assert_eq!(parse_peer_urls("   "), Vec::<String>::new());
    }
}
