//! Entropy beacon — the external randomness that seeds each block.
//!
//! The live beacon ([`sample_live`]) is [drand](https://drand.love)'s public
//! `quicknet` randomness network — a threshold-BLS beacon run by independent
//! operators (the League of Entropy), publishing a fresh, unbiasable, publicly
//! verifiable random value every 3 seconds. No single party (including us) can
//! predict or influence it. [`sample`] is the deterministic offline fallback: used
//! in tests, and live if drand is unreachable, so the chain keeps running instead of
//! stalling.

use serde::Deserialize;

/// drand's `quicknet` chain — 3s rounds, matches Entropa's own block cadence.
const DRAND_QUICKNET_URL: &str =
    "https://api.drand.sh/52db9ba70e0cc0f6eaf7803dd07447a1f5477735fd3f661792ba94600c84e971/public/latest";

#[derive(Deserialize)]
struct DrandRound {
    round: u64,
    randomness: String,
}

/// Fetch the current round from drand's public `quicknet` beacon. Returns `None` on
/// any failure (network, bad status, malformed JSON) — the caller falls back to
/// [`sample`].
pub async fn sample_live() -> Option<String> {
    let resp = reqwest::Client::new()
        .get(DRAND_QUICKNET_URL)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let round: DrandRound = resp.json().await.ok()?;
    Some(format!(
        "DRAND-{}-{}",
        round.round,
        &round.randomness[..16]
    ))
}

/// Deterministic offline fallback — derives a value from the round number. Used in
/// tests (no network dependency) and if [`sample_live`] can't reach drand. Prefixed
/// differently from [`sample_live`]'s output so it's visible on-chain which mode
/// produced a given block's beacon.
pub fn sample(round: u64) -> String {
    let digest = blake3::hash(&round.to_be_bytes());
    format!("BEACON-{}", &digest.to_hex()[..16])
}
