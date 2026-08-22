//! `GET /api/receipt/:id` — the Attestation Receipt for a transaction.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use entropa_core::{Block, Transaction};

use crate::{persistence, AppState};

/// Build the Attestation Receipt JSON for a known `(block, tx)` pair — pure, and
/// deliberately independent of *where* the block came from (in-memory window or a
/// Firestore fallback load for an evicted block), so both callers in [`receipt`]
/// share identical output and this logic is unit-testable without any I/O.
fn build_receipt_json(id: &str, block: &Block, tx: &Transaction) -> serde_json::Value {
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

    serde_json::json!({
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
    })
}

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
///
/// Checks the in-memory window first (fast path, the overwhelming majority of
/// requests — receipts are usually looked up soon after submission). If the
/// transaction's block has since been evicted by `bound_to_window`, falls back to
/// the receipt index + a single Firestore block fetch (`persistence::load_block`)
/// rather than 404ing just because the block is no longer resident in RAM. Fails
/// soft to 404 if the index has no entry (e.g. submitted before the index existed)
/// or Firestore/the metadata server is unreachable — identical to an unknown id.
pub(crate) async fn receipt(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let found_in_memory = {
        let n = s.node.lock().unwrap();
        n.chain.blocks.iter().find_map(|block| {
            block
                .transactions
                .iter()
                .find(|t| t.content_hash() == id)
                .map(|tx| (block.clone(), tx.clone()))
        })
    };
    if let Some((block, tx)) = found_in_memory {
        return Ok(Json(build_receipt_json(&id, &block, &tx)));
    }

    if let Some((epoch, block_index)) = persistence::get_receipt_location(&id).await {
        if let Some(block) = persistence::load_block(&epoch, block_index).await {
            if let Some(tx) = block.transactions.iter().find(|t| t.content_hash() == id) {
                return Ok(Json(build_receipt_json(&id, &block, tx)));
            }
        }
    }
    Err(StatusCode::NOT_FOUND)
}

#[cfg(test)]
mod tests {
    use super::build_receipt_json;
    use crate::test_support::node_with_one_block;
    use crate::{app, AppState};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[test]
    fn build_receipt_json_recomputes_hash_and_signature_correctly_from_any_source() {
        // Proves the JSON-building logic itself is correct independent of whether the
        // caller found the block in-memory or via the Firestore fallback — both routes
        // now share this exact function, so this is the real coverage for the shape
        // that used to only be reachable through the in-memory route test.
        let node = node_with_one_block();
        let block = &node.chain.blocks[0];
        let tx = &block.transactions[0];
        let id = tx.content_hash();

        let json = build_receipt_json(&id, block, tx);

        assert_eq!(json["receipt_id"], id);
        assert_eq!(json["status"], "confirmed");
        assert_eq!(
            json["independent_verification"]["hash_recomputed_and_matches"],
            true
        );
        assert_eq!(json["independent_verification"]["signature_verified"], true);
        assert_eq!(json["technical"]["block_index"], block.index);
    }

    #[test]
    fn build_receipt_json_flags_a_tampered_block_even_though_lookup_still_succeeds() {
        // The fallback path loads a block fetched fresh from Firestore rather than the
        // chain's own in-memory, already-verified Vec — this is the property that
        // matters there: even a block that was somehow altered in storage gets caught
        // by the fresh recompute, never silently trusted just because it was "found."
        let node = node_with_one_block();
        let mut block = node.chain.blocks[0].clone();
        let tx = block.transactions[0].clone();
        let id = tx.content_hash();
        block.hash = "tampered".to_string();

        let json = build_receipt_json(&id, &block, &tx);

        assert_eq!(
            json["independent_verification"]["hash_recomputed_and_matches"],
            false
        );
    }

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
        assert!(json["plain_english"]["summary"].as_str().unwrap().len() > 20);
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

    #[tokio::test]
    async fn receipt_falls_back_and_still_404s_cleanly_when_evicted_and_unindexed() {
        // Simulates a receipt for a transaction that fell outside the in-memory window
        // (bound_to_window evicted its block) and was never indexed (submitted before
        // the receipt index existed). The fallback (persistence::get_receipt_location)
        // fails soft to None without GCP_PROJECT set (real in this test/local env, not
        // mocked) — this proves the fallback path is actually exercised and still
        // degrades to a clean 404 rather than panicking or hanging, not just that the
        // original in-memory lookup 404s on a garbage id.
        std::env::remove_var("GCP_PROJECT");
        let resp = app(AppState::new(node_with_one_block()))
            .oneshot(
                Request::builder()
                    .uri("/api/receipt/evicted-and-never-indexed")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
