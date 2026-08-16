//! `GET /api/health`, `GET /api/chain`, `GET /api/head` — read-only chain data endpoints.

use axum::extract::{Query, State};
use axum::Json;

use entropa_core::Block;

use crate::AppState;

pub(crate) async fn health(State(s): State<AppState>) -> Json<serde_json::Value> {
    let n = s.node.lock().unwrap();
    Json(serde_json::json!({
        "status": "ok",
        "network": "Entropa",
        "tagline": "Entropa is boring. All we do is keep your AI agents auditable. For pennies. ($0.01/attestation)",
        "consensus": entropa_node::consensus::NAME,
        "signature": "ML-DSA (NIST FIPS-204)",
        "height": n.height(),
    }))
}

/// `?limit=` (default 1000, capped at 2000) and `?offset=` (default: the tail of the
/// chain, i.e. the most recent blocks) — the full chain is too large for one Cloud Run
/// response once it grows past a few thousand blocks; walk it with `offset` for history.
#[derive(serde::Deserialize)]
pub(crate) struct ChainQuery {
    limit: Option<usize>,
    offset: Option<usize>,
}

pub(crate) async fn chain(
    State(s): State<AppState>,
    Query(q): Query<ChainQuery>,
) -> Json<Vec<Block>> {
    let n = s.node.lock().unwrap();
    let total = n.chain.blocks.len();
    let limit = q.limit.unwrap_or(1000).min(2000);
    let offset = q.offset.unwrap_or_else(|| total.saturating_sub(limit));
    let end = (offset + limit).min(total);
    let page = n.chain.blocks.get(offset.min(total)..end).unwrap_or(&[]);
    Json(page.to_vec())
}

pub(crate) async fn head(State(s): State<AppState>) -> Json<Option<Block>> {
    let n = s.node.lock().unwrap();
    Json(n.chain.head().cloned())
}

#[cfg(test)]
mod tests {
    use crate::test_support::node_with_one_block;
    use crate::{app, AppState};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use entropa_core::{Block, Probe};
    use entropa_node::{Node, Validator};
    use tower::ServiceExt;

    #[tokio::test]
    async fn health_ok() {
        let resp = app(AppState::new(node_with_one_block()))
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn healthz_alias_matches_api_health() {
        let resp = app(AppState::new(node_with_one_block()))
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn chain_returns_ok() {
        let resp = app(AppState::new(node_with_one_block()))
            .oneshot(
                Request::builder()
                    .uri("/api/chain")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    /// Regression test for the 2026-08-12 incident: `/api/chain` used to dump the entire
    /// chain unpaginated, which exceeded Cloud Run's response-size limit once the live
    /// chain passed a few thousand blocks. Builds a chain well past the default page size
    /// and asserts the default response stays bounded, proving pagination actually caps
    /// it rather than just being available as an unused option.
    #[tokio::test]
    async fn chain_default_response_is_bounded() {
        let probe = Probe::spawn();
        let validators = vec![Validator::new(probe.id(), probe.pubkey_hex())];
        let mut node = Node::new(probe, validators);
        for i in 0..1200u64 {
            node.try_produce(i, 1_000 + i);
        }
        assert!(
            node.chain.blocks.len() > 1000,
            "test setup must exceed the default page size"
        );

        let resp = app(AppState::new(node))
            .oneshot(
                Request::builder()
                    .uri("/api/chain")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let blocks: Vec<Block> = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            blocks.len(),
            1000,
            "default /api/chain response must stay capped even when the chain is much larger"
        );
    }
}
