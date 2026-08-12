//! Cosmic entropy beacon — the space-sourced randomness that seeds each block.
//!
//! This is where `.space` becomes a *feature*, not decoration. Entropa fights entropy;
//! it also *harvests* it. Each round draws an unbiasable random value from a public
//! space / astronomy randomness source, runs it through a VRF, and commits the result
//! on-chain as the seed for proposer (Probe) selection. Order, drawn from the entropy
//! of space.
//!
//! Production will fetch a live cosmic beacon (space-weather / pulsar-timing / a
//! drand-style beacon) and attach a VRF proof. For now this is a deterministic stub so
//! the `beacon` field is structural from block zero — the wiring point is one function.

/// Sample the cosmic entropy beacon for a given round.
///
/// Stub: derives a value from the round number. Real impl fetches a space beacon and
/// returns `COSMIC-<vrf-output>` with a verifiable proof.
pub fn sample(round: u64) -> String {
    let digest = blake3::hash(&round.to_be_bytes());
    format!("COSMIC-{}", &digest.to_hex()[..16])
}
