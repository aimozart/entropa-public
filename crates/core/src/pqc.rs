//! Post-quantum identity + signatures — ML-DSA (NIST FIPS 204).
//!
//! Every actor on Entropa is a **Probe**: an autonomous agent that proposes and
//! validates blocks. A Probe's identity is an ML-DSA-65 keypair. Its public
//! *fingerprint* is a blake3 hash of the verifying key, rendered as `PROBE-XXXXXXXX`
//! — short enough to read on the star-map, unforgeable because it's bound to the key.
//!
//! Signing and verifying use post-quantum lattice cryptography, so the chain's
//! authenticity survives a future quantum adversary — the whole point of Entropa.

use ml_dsa::signature::{Signer, Verifier};
use ml_dsa::{
    EncodedSignature, EncodedVerifyingKey, Generate, Keypair, MlDsa65, Signature, SigningKey,
    VerifyingKey,
};

/// A Probe — a post-quantum identity that can sign blocks and transactions.
pub struct Probe {
    signing: SigningKey<MlDsa65>,
}

impl Probe {
    /// Spawn a fresh Probe with a new ML-DSA-65 keypair.
    pub fn spawn() -> Self {
        Self {
            signing: SigningKey::<MlDsa65>::generate(),
        }
    }

    /// The Probe's verifying (public) key.
    pub fn verifying_key(&self) -> VerifyingKey<MlDsa65> {
        self.signing.verifying_key()
    }

    /// Hex-encoded ML-DSA verifying key — the Probe's on-chain public identity.
    pub fn pubkey_hex(&self) -> String {
        hex::encode(self.verifying_key().encode())
    }

    /// Human-readable fingerprint, e.g. `PROBE-1A2B3C4D`.
    pub fn id(&self) -> String {
        probe_id(&self.pubkey_hex())
    }

    /// Post-quantum sign `msg`, returning a hex signature.
    pub fn sign_hex(&self, msg: &[u8]) -> String {
        let sig: Signature<MlDsa65> = self.signing.sign(msg);
        hex::encode(sig.encode())
    }

    /// Reconstruct the exact Probe a prior [`Probe::signing_key_hex`] export came
    /// from. Needed so a validator's identity survives process restarts/redeploys —
    /// generate once, store the hex string as a secret, load it every boot instead
    /// of calling [`Probe::spawn`]. `None` on any decode failure, including the
    /// wrong byte length — never panics on bad input.
    pub fn from_signing_key_hex(hex_seed: &str) -> Option<Self> {
        let bytes = hex::decode(hex_seed).ok()?;
        let seed_bytes: [u8; 32] = bytes.try_into().ok()?;
        Some(Self {
            signing: SigningKey::<MlDsa65>::from_seed(&seed_bytes.into()),
        })
    }

    /// Hex-encoded 32-byte seed that reconstructs this exact Probe via
    /// [`Probe::from_signing_key_hex`]. This is key material — treat it like a
    /// password (store as a secret, never log it).
    pub fn signing_key_hex(&self) -> String {
        hex::encode(&self.signing.as_seed()[..])
    }
}

/// Derive a Probe's short fingerprint from its hex public key.
pub fn probe_id(pubkey_hex: &str) -> String {
    let digest = blake3::hash(pubkey_hex.as_bytes());
    format!("PROBE-{}", digest.to_hex()[..8].to_uppercase())
}

/// Verify a hex ML-DSA signature over `msg` against a hex verifying key.
/// Returns `false` on any decode failure or signature mismatch — never panics.
pub fn verify_hex(pubkey_hex: &str, msg: &[u8], sig_hex: &str) -> bool {
    let Ok(pk_bytes) = hex::decode(pubkey_hex) else {
        return false;
    };
    let Ok(sig_bytes) = hex::decode(sig_hex) else {
        return false;
    };
    let Ok(enc_vk) = EncodedVerifyingKey::<MlDsa65>::try_from(pk_bytes.as_slice()) else {
        return false;
    };
    let Ok(enc_sig) = EncodedSignature::<MlDsa65>::try_from(sig_bytes.as_slice()) else {
        return false;
    };
    let vk = VerifyingKey::<MlDsa65>::decode(&enc_vk);
    let Some(sig) = Signature::<MlDsa65>::decode(&enc_sig) else {
        return false;
    };
    vk.verify(msg, &sig).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_and_verify_round_trip() {
        let probe = Probe::spawn();
        let msg = b"finalize block 42";
        let sig = probe.sign_hex(msg);
        assert!(verify_hex(&probe.pubkey_hex(), msg, &sig));
    }

    #[test]
    fn rejects_tampered_message() {
        let probe = Probe::spawn();
        let sig = probe.sign_hex(b"pay 10 to alice");
        assert!(!verify_hex(&probe.pubkey_hex(), b"pay 99 to alice", &sig));
    }

    #[test]
    fn rejects_wrong_key() {
        let a = Probe::spawn();
        let b = Probe::spawn();
        let sig = a.sign_hex(b"hello");
        assert!(!verify_hex(&b.pubkey_hex(), b"hello", &sig));
    }

    /// Load-bearing for the planned VRF-style selection proof (see
    /// docs/plans — "Real Quorum Consensus"): a signature can only double as a
    /// deterministic selection input if signing itself is deterministic. This
    /// test is the single source of truth for that fact — don't assume either
    /// way, and don't build selection logic on top of this until it's green.
    #[test]
    fn sign_hex_determinism_check() {
        let probe = Probe::spawn();
        let msg = b"COSMIC-round-determinism-check";
        let sig1 = probe.sign_hex(msg);
        let sig2 = probe.sign_hex(msg);
        println!(
            "sign_hex determinism: {}",
            if sig1 == sig2 {
                "DETERMINISTIC"
            } else {
                "RANDOMIZED/HEDGED"
            }
        );
        // Intentionally no assertion on equality — this test exists to observe and
        // report the fact, not to enforce an assumption about it.
        assert!(verify_hex(&probe.pubkey_hex(), msg, &sig1));
        assert!(verify_hex(&probe.pubkey_hex(), msg, &sig2));
    }

    #[test]
    fn id_is_stable_and_prefixed() {
        let probe = Probe::spawn();
        assert_eq!(probe.id(), probe.id());
        assert!(probe.id().starts_with("PROBE-"));
        assert_eq!(probe.id().len(), "PROBE-".len() + 8);
    }

    // --- Phase 4a: persisted identity ---
    //
    // `Probe::spawn()` alone means a validator's identity changes on every process
    // restart — harmless for a single-validator network, but fatal for N=3 quorum,
    // where the other 2 validators' configured pubkeys go stale on every redeploy of
    // the third. A key exported once and handed to Cloud Run as a secret must
    // reconstruct the exact same signing identity every time.

    #[test]
    fn exported_key_reconstructs_the_same_identity() {
        let original = Probe::spawn();
        let exported = original.signing_key_hex();
        let restored = Probe::from_signing_key_hex(&exported).expect("valid exported key");
        assert_eq!(original.id(), restored.id());
        assert_eq!(original.pubkey_hex(), restored.pubkey_hex());
    }

    #[test]
    fn restored_probe_signs_identically_to_the_original() {
        // sign_hex is confirmed deterministic (sign_hex_determinism_check) - the
        // restored key must therefore produce byte-for-byte the same signature as
        // the original for the same message, not just a signature that happens to
        // verify.
        let original = Probe::spawn();
        let restored = Probe::from_signing_key_hex(&original.signing_key_hex()).unwrap();
        let msg = b"round 42 beacon value";
        assert_eq!(original.sign_hex(msg), restored.sign_hex(msg));
    }

    #[test]
    fn malformed_hex_is_rejected() {
        assert!(Probe::from_signing_key_hex("not-hex-at-all").is_none());
    }

    #[test]
    fn wrong_length_key_is_rejected() {
        // A valid hex string, but not the required 32 raw bytes.
        assert!(Probe::from_signing_key_hex("aabbcc").is_none());
    }
}
