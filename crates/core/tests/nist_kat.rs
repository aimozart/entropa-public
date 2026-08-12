//! NIST FIPS-204 known-answer tests for ML-DSA-65 — Entropa's post-quantum signature scheme.
//!
//! These assert that the cryptography under Entropa produces **byte-for-byte identical**
//! output to NIST's own reference vectors. Not "a signature that verifies" — the exact
//! bytes NIST says the algorithm must emit, for key generation and for both signing
//! interfaces, plus verification of NIST's own signatures and rejection of tampered ones.
//!
//! Vectors are official ACVP files, committed verbatim. Provenance and a script to run
//! against the complete upstream set: `tests/vectors/README.md`.
//!
//! **This is a correctness artifact, not a certification.** Entropa is not FIPS-validated
//! and claims no such thing — formal validation requires an accredited CMVP/CAVP lab.

// ACVP encodes signing keys in *expanded* form (4032 bytes for ML-DSA-65). `from_expanded`
// / `to_expanded` are the only APIs that speak that format, so interoperating with the
// official vectors requires them despite their deprecation in favour of seed-based keys.
#![allow(deprecated)]

use ml_dsa::{
    EncodedSignature, ExpandedSigningKey, ExpandedSigningKeyBytes, Keypair, MlDsa65, Signature,
    SigningKey, B32,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct Vectors {
    #[serde(rename = "keyGen")]
    key_gen: Vec<KeyGenCase>,
    #[serde(rename = "sigGenInternalDeterministic")]
    sig_gen_internal: Vec<SigGenInternalCase>,
    #[serde(rename = "sigGenExternalPureDeterministic")]
    sig_gen_external: Vec<SigGenExternalCase>,
}

#[derive(Deserialize)]
struct KeyGenCase {
    #[serde(rename = "tcId")]
    tc_id: u32,
    /// The 32-byte seed ξ.
    seed: String,
    pk: String,
    sk: String,
}

#[derive(Deserialize)]
struct SigGenInternalCase {
    #[serde(rename = "tcId")]
    tc_id: u32,
    message: String,
    sk: String,
    signature: String,
}

#[derive(Deserialize)]
struct SigGenExternalCase {
    #[serde(rename = "tcId")]
    tc_id: u32,
    message: String,
    context: String,
    sk: String,
    signature: String,
}

fn vectors() -> Vectors {
    serde_json::from_str(include_str!("vectors/ml_dsa_65_acvp.json"))
        .expect("NIST ACVP vector file parses")
}

fn unhex(s: &str) -> Vec<u8> {
    hex::decode(s).expect("valid hex in NIST vector")
}

/// Load an ACVP expanded signing key (the `sk` field) into a usable key.
fn load_sk(sk_hex: &str) -> ExpandedSigningKey<MlDsa65> {
    let bytes = unhex(sk_hex);
    let encoded = ExpandedSigningKeyBytes::<MlDsa65>::try_from(bytes.as_slice())
        .expect("ACVP sk is exactly 4032 bytes for ML-DSA-65");
    ExpandedSigningKey::<MlDsa65>::from_expanded(&encoded)
}

/// Decode an ACVP signature (the `signature` field) into a usable signature.
fn load_sig(sig_hex: &str) -> Signature<MlDsa65> {
    let bytes = unhex(sig_hex);
    let encoded = EncodedSignature::<MlDsa65>::try_from(bytes.as_slice())
        .expect("ACVP signature is exactly 3309 bytes for ML-DSA-65");
    Signature::<MlDsa65>::decode(&encoded).expect("NIST signature decodes")
}

/// `ML-DSA.KeyGen_internal` — seed ξ must expand to exactly NIST's (pk, sk).
#[test]
fn nist_keygen_matches_reference_bytes() {
    let v = vectors();
    assert!(!v.key_gen.is_empty(), "no keyGen vectors loaded");

    for case in &v.key_gen {
        let seed_bytes = unhex(&case.seed);
        let seed = B32::try_from(seed_bytes.as_slice()).expect("seed ξ is 32 bytes");
        let sk = SigningKey::<MlDsa65>::from_seed(&seed);

        assert_eq!(
            hex::encode_upper(sk.verifying_key().encode()),
            case.pk.to_uppercase(),
            "keyGen tcId {}: public key does not match NIST reference",
            case.tc_id
        );
        assert_eq!(
            hex::encode_upper(sk.expanded_key().to_expanded()),
            case.sk.to_uppercase(),
            "keyGen tcId {}: expanded signing key does not match NIST reference",
            case.tc_id
        );
    }

    println!(
        "✓ {} NIST keyGen vectors matched byte-for-byte",
        v.key_gen.len()
    );
}

/// `ML-DSA.Sign_internal`, deterministic variant (rnd = 0³²).
#[test]
fn nist_sign_internal_matches_reference_bytes() {
    let v = vectors();
    assert!(
        !v.sig_gen_internal.is_empty(),
        "no internal sigGen vectors loaded"
    );

    // The deterministic variant fixes the per-signature randomness to all zeros.
    let rnd = B32::default();

    for case in &v.sig_gen_internal {
        let esk = load_sk(&case.sk);
        let message = unhex(&case.message);
        let sig = esk.sign_internal(&[message.as_slice()], &rnd);

        assert_eq!(
            hex::encode_upper(sig.encode()),
            case.signature.to_uppercase(),
            "sigGen(internal) tcId {}: signature does not match NIST reference",
            case.tc_id
        );
    }

    println!(
        "✓ {} NIST sigGen (internal, deterministic) vectors matched byte-for-byte",
        v.sig_gen_internal.len()
    );
}

/// `ML-DSA.Sign` — the full external path, including the context string.
#[test]
fn nist_sign_external_pure_matches_reference_bytes() {
    let v = vectors();
    assert!(
        !v.sig_gen_external.is_empty(),
        "no external sigGen vectors loaded"
    );

    for case in &v.sig_gen_external {
        let esk = load_sk(&case.sk);
        let message = unhex(&case.message);
        let context = unhex(&case.context);
        let sig = esk
            .sign_deterministic(&message, &context)
            .expect("deterministic signing succeeds");

        assert_eq!(
            hex::encode_upper(sig.encode()),
            case.signature.to_uppercase(),
            "sigGen(external/pure) tcId {}: signature does not match NIST reference",
            case.tc_id
        );
    }

    println!(
        "✓ {} NIST sigGen (external/pure, deterministic) vectors matched byte-for-byte",
        v.sig_gen_external.len()
    );
}

/// Verification accepts NIST's own signatures — proving the verify path, not just signing.
#[test]
fn verifies_nist_reference_signatures() {
    let v = vectors();

    for case in &v.sig_gen_external {
        let vk = load_sk(&case.sk).verifying_key();
        let message = unhex(&case.message);
        let context = unhex(&case.context);
        let sig = load_sig(&case.signature);

        assert!(
            vk.verify_with_context(&message, &context, &sig),
            "sigGen(external/pure) tcId {}: failed to verify NIST's own signature",
            case.tc_id
        );
    }

    for case in &v.sig_gen_internal {
        let vk = load_sk(&case.sk).verifying_key();
        let message = unhex(&case.message);
        let sig = load_sig(&case.signature);

        assert!(
            vk.verify_internal(&message, &sig),
            "sigGen(internal) tcId {}: failed to verify NIST's own signature",
            case.tc_id
        );
    }

    println!(
        "✓ verified {} NIST reference signatures",
        v.sig_gen_external.len() + v.sig_gen_internal.len()
    );
}

/// The security-critical direction: a tampered signature, message, or context must fail.
#[test]
fn rejects_tampered_nist_signatures() {
    let v = vectors();

    for case in &v.sig_gen_external {
        let vk = load_sk(&case.sk).verifying_key();
        let message = unhex(&case.message);
        let context = unhex(&case.context);

        // Flip one bit in the signature.
        let mut sig_bytes = unhex(&case.signature);
        sig_bytes[0] ^= 0x01;
        if let Ok(encoded) = EncodedSignature::<MlDsa65>::try_from(sig_bytes.as_slice()) {
            if let Some(bad_sig) = Signature::<MlDsa65>::decode(&encoded) {
                assert!(
                    !vk.verify_with_context(&message, &context, &bad_sig),
                    "tcId {}: accepted a signature with a flipped bit",
                    case.tc_id
                );
            }
            // A decode failure is also a valid rejection — the tampering was caught earlier.
        }

        // Flip one bit in the message.
        let good_sig = load_sig(&case.signature);
        let mut tampered_msg = message.clone();
        if let Some(b) = tampered_msg.first_mut() {
            *b ^= 0x01;
            assert!(
                !vk.verify_with_context(&tampered_msg, &context, &good_sig),
                "tcId {}: accepted a signature over a modified message",
                case.tc_id
            );
        }

        // Change the context — a signature is bound to its context string.
        let mut tampered_ctx = context.clone();
        tampered_ctx.push(0xAB);
        assert!(
            !vk.verify_with_context(&message, &tampered_ctx, &good_sig),
            "tcId {}: accepted a signature under a different context",
            case.tc_id
        );
    }

    println!(
        "✓ rejected tampered signatures, messages, and contexts across {} vectors",
        v.sig_gen_external.len()
    );
}
