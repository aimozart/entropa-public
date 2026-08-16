//! `POST /api/tx` — submit a transaction into the mempool.

use axum::{extract::State, http::HeaderMap, http::StatusCode, BoxError, Json};

use entropa_core::Transaction;

use crate::state::check_per_key_rate_limit;
use crate::AppState;

/// Turns a buffered/rate-limited request's failure (queue full, or over the rate cap)
/// into a proper HTTP response instead of the connection just dying.
pub(crate) async fn rate_limit_error(err: BoxError) -> (StatusCode, String) {
    (
        StatusCode::TOO_MANY_REQUESTS,
        format!("rate limited: {err}"),
    )
}

/// Pull the bearer token out of `Authorization: Bearer <key>`, if present.
fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
}

pub(crate) async fn submit_tx(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(mut tx): Json<Transaction>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !s.api_keys.is_empty() {
        let partner = bearer_token(&headers)
            .and_then(|token| s.api_keys.get(token))
            .ok_or(StatusCode::UNAUTHORIZED)?;
        if !check_per_key_rate_limit(&s.rate_buckets, partner) {
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
        // The authenticated key's owner overrides whatever `from` the client sent —
        // a partner can't submit a transaction claiming to be someone else.
        tx.from = partner.clone();
    }
    let receipt_id = tx.content_hash();
    let mut n = s.node.lock().unwrap();
    n.submit(tx);
    Ok(Json(serde_json::json!({
        "accepted": true,
        "pending": n.mempool.len(),
        "receipt_id": receipt_id,
        "receipt_note": "Not yet proof of anything — this only confirms your transaction is \
             queued. Within a few seconds it will be included in a signed block; poll \
             GET /api/receipt/{receipt_id} to get the actual Attestation Receipt.",
    })))
}

#[cfg(test)]
mod tests {
    use crate::test_support::node_with_one_block;
    use crate::{app, AppState};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use std::sync::Arc;
    use tower::ServiceExt;

    #[tokio::test]
    async fn tx_response_includes_receipt_id() {
        let resp = app(AppState::new(node_with_one_block()))
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tx")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"from":"anyone","kind":"attest","payload":"hi"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(!json["receipt_id"].as_str().unwrap().is_empty());
    }

    #[tokio::test]
    async fn tx_open_when_no_keys_configured() {
        let resp = app(AppState::new(node_with_one_block()))
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tx")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"from":"anyone","kind":"attest","payload":"hi"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn tx_rejects_missing_key_when_keys_configured() {
        let state = AppState::new(node_with_one_block()).with_api_keys("acme:secret123");
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tx")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"from":"anyone","kind":"attest","payload":"hi"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn tx_accepts_valid_key_and_overrides_from() {
        let state = AppState::new(node_with_one_block()).with_api_keys("acme:secret123");
        let n = Arc::clone(&state.node);
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tx")
                    .header("content-type", "application/json")
                    .header("authorization", "Bearer secret123")
                    .body(Body::from(
                        r#"{"from":"spoofed","kind":"attest","payload":"hi"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let locked = n.lock().unwrap();
        let pending = locked.mempool.pending();
        assert_eq!(pending.len(), 1);
        assert_eq!(
            pending[0].from, "acme",
            "authenticated `from` must win over the client-supplied one"
        );
    }
}
