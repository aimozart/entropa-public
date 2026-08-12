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
}

/// Derive a Probe's short fingerprint from its hex public key.
pub fn probe_id(pubkey_hex: &str) -> String {
    let digest = blake3::hash(pubkey_hex.as_bytes());
    format!("PROBE-{}", &digest.to_hex()[..8].to_uppercase())
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

    #[test]
    fn id_is_stable_and_prefixed() {
        let probe = Probe::spawn();
        assert_eq!(probe.id(), probe.id());
        assert!(probe.id().starts_with("PROBE-"));
        assert_eq!(probe.id().len(), "PROBE-".len() + 8);
    }
}
