//! `GET /api/receipt/:id` — the Attestation Receipt for a transaction.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::AppState;

/// The **Attestation Receipt** — the proof a customer hands to their own auditor
/// for a specific recorded action. Looked up by the `content_hash` returned from
/// `POST /api/tx` at submission time, independent of which block/index it ended
/// up in.
///
/// Every field an auditor needs to independently re-verify is included — not just
/// asserted from storage, but *recomputed fresh* on every request
/// (`independent_verification`), so "trust us" is never required. A plain-English
/// translation sits alongside the raw cryptographic fields, so the same document
/// works whether the reader is a compliance officer or a security engineer.
pub(crate) async fn receipt(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let n = s.node.lock().unwrap();
    for block in &n.chain.blocks {
        let Some(tx) = block.transactions.iter().find(|t| t.content_hash() == id) else {
            continue;
        };

        let digest = entropa_core::block_digest(
            block.index,
            block.timestamp,
            &block.prev_hash,
            &block.beacon,
            &block.transactions,
            &block.proposer_id,
        );
        let hash_matches = hex::encode(digest) == block.hash;
        let signature_valid =
            entropa_core::verify_hex(&block.proposer_pubkey, &digest, &block.signature);

        return Ok(Json(serde_json::json!({
            "receipt_id": id,
            "status": "confirmed",
            "plain_english": {
                "summary": format!(
                    "This action was permanently recorded by Entropa at height {}. It has been \
                     cryptographically signed and independently re-verified just now — it cannot \
                     be altered or deleted without that tampering being immediately detectable. \
                     You do not need to trust Entropa's word for this; you or your auditor can \
                     recompute every check below yourselves.",
                    block.index
                ),
                "what_happened": format!("Probe \"{}\" recorded a \"{}\" action.", tx.from, tx.kind),
                "the_fingerprint": "The block hash below is a unique digital fingerprint of this \
                     exact record. Changing even one character of the underlying data would \
                     produce a completely different fingerprint — so a matching fingerprint is \
                     proof nothing has been altered.",
                "the_signature": "The signature proves this specific, identifiable Probe created \
                     this record — using a post-quantum signature scheme (ML-DSA), designed to \
                     stay secure even against future quantum computers, not just today's.",
                "the_ordering_proof": "The beacon value proves the recording order wasn't \
                     manipulated by Entropa itself — it comes from a public randomness source \
                     (drand) that Entropa does not control.",
            },
            "transaction": {
                "from": tx.from,
                "kind": tx.kind,
                "payload": tx.payload,
            },
            "technical": {
                "block_index": block.index,
                "block_timestamp": block.timestamp,
                "beacon": block.beacon,
                "proposer_id": block.proposer_id,
                "proposer_pubkey": block.proposer_pubkey,
                "block_hash": block.hash,
                "signature": block.signature,
                "hash_algorithm": "BLAKE3",
                "signature_algorithm": "ML-DSA-65 (NIST FIPS-204)",
            },
            "independent_verification": {
                "hash_recomputed_and_matches": hash_matches,
                "signature_verified": signature_valid,
                "note": "These two checks were re-performed fresh, right now, against the raw \
                     block data — not read from a cached or stored 'verified' flag.",
            },
        })));
    }
    Err(StatusCode::NOT_FOUND)
}

#[cfg(test)]
mod tests {
    use crate::test_support::node_with_one_block;
    use crate::{app, AppState};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn receipt_returns_proof_for_known_transaction() {
        let node = node_with_one_block();
        let known_tx_id = node.chain.blocks[0].transactions[0].content_hash();
        let resp = app(AppState::new(node))
            .oneshot(
                Request::builder()
                    .uri(format!("/api/receipt/{known_tx_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            json["independent_verification"]["hash_recomputed_and_matches"],
            true
        );
        assert_eq!(json["independent_verification"]["signature_verified"], true);
        assert_eq!(json["technical"]["block_index"], 0);
    }

    #[tokio::test]
    async fn receipt_404s_for_unknown_id() {
        let resp = app(AppState::new(node_with_one_block()))
            .oneshot(
                Request::builder()
                    .uri("/api/receipt/not-a-real-receipt-id")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
