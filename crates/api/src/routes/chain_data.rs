//! `GET /api/health`, `GET /api/chain`, `GET /api/head` — read-only chain data endpoints.

use axum::extract::{Query, State};
use axum::Json;

use entropa_core::Block;

use crate::{persistence, AppState};

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

/// Pure: split a requested `[offset, end)` page of *global* block indices into the
/// portion that has aged out of the in-memory window (`below`, indices `< base_index`,
/// must come from Firestore) and the portion still resident in RAM (`mem`). No I/O —
/// the async fallback loop in [`chain`] is a thin wrapper around this.
///
/// `base_index.clamp(offset, end)` is the split point: if the window hasn't evicted
/// anything relevant to this page (`base_index <= offset`) or has evicted the whole
/// page (`base_index >= end`), one side comes back empty rather than needing a
/// separate branch for each case.
fn split_page_bounds(
    offset: usize,
    end: usize,
    base_index: usize,
) -> (std::ops::Range<usize>, std::ops::Range<usize>) {
    let split = base_index.clamp(offset, end);
    (offset..split, split..end)
}

pub(crate) async fn chain(
    State(s): State<AppState>,
    Query(q): Query<ChainQuery>,
) -> Json<Vec<Block>> {
    let (below, mem_blocks, epoch) = {
        let n = s.node.lock().unwrap();
        let total = n.chain.height() as usize;
        let base_index = n.chain.base_index as usize;
        let limit = q.limit.unwrap_or(1000).min(2000);
        let offset = q.offset.unwrap_or_else(|| total.saturating_sub(limit));
        let end = (offset + limit).min(total);
        let (below, mem) = split_page_bounds(offset, end, base_index);
        let mem_blocks = if mem.is_empty() {
            Vec::new()
        } else {
            n.chain
                .blocks
                .get((mem.start - base_index)..(mem.end - base_index))
                .unwrap_or(&[])
                .to_vec()
        };
        (below, mem_blocks, s.active_epoch.clone())
    };

    let mut page = Vec::with_capacity(below.len() + mem_blocks.len());
    if !below.is_empty() {
        if let Some(epoch) = epoch {
            for idx in below {
                if let Some(block) = persistence::load_block_in_lineage(&epoch, idx as u64).await {
                    page.push(block);
                }
            }
        }
    }
    page.extend(mem_blocks);
    Json(page)
}

pub(crate) async fn head(State(s): State<AppState>) -> Json<Option<Block>> {
    let n = s.node.lock().unwrap();
    Json(n.chain.head().cloned())
}

#[cfg(test)]
mod tests {
    use super::split_page_bounds;
    use crate::test_support::node_with_one_block;
    use crate::{app, AppState};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use entropa_core::{Block, Probe};
    use entropa_node::{Node, Validator};
    use tower::ServiceExt;

    #[test]
    fn split_page_bounds_is_all_in_memory_when_nothing_has_been_evicted() {
        assert_eq!(split_page_bounds(0, 1000, 0), (0..0, 0..1000));
    }

    #[test]
    fn split_page_bounds_is_all_below_floor_when_the_whole_page_predates_it() {
        assert_eq!(split_page_bounds(0, 500, 1000), (0..500, 500..500));
    }

    #[test]
    fn split_page_bounds_splits_a_page_straddling_the_floor() {
        assert_eq!(split_page_bounds(500, 1500, 1000), (500..1000, 1000..1500));
    }

    #[test]
    fn split_page_bounds_is_all_in_memory_when_the_page_starts_after_the_floor() {
        assert_eq!(
            split_page_bounds(2000, 3000, 1000),
            (2000..2000, 2000..3000)
        );
    }

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
    async fn chain_page_below_the_window_floor_degrades_gracefully_without_firestore() {
        // Simulates a request for history that has aged out of the in-memory window
        // (bound_to_window). Without GCP_PROJECT set (real in this test/local env),
        // the Firestore fallback fails soft — this proves the route returns an empty
        // page rather than panicking, erroring, or (worse) silently substituting
        // wrong data for a range it genuinely can't serve right now.
        std::env::remove_var("GCP_PROJECT");
        let probe = Probe::spawn();
        let validators = vec![Validator::new(probe.id(), probe.pubkey_hex())];
        let mut node = Node::new(probe, validators);
        for i in 0..50u64 {
            node.try_produce(i, 1_000 + i);
        }
        node.chain = node.chain.clone().bound_to_window(10);
        assert_eq!(
            node.chain.base_index, 40,
            "test setup: floor must sit at height 40"
        );

        let resp = app(AppState::new(node).with_active_epoch("test-epoch"))
            .oneshot(
                Request::builder()
                    .uri("/api/chain?offset=0&limit=10")
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
        assert!(
            blocks.is_empty(),
            "a fully-evicted range with no reachable Firestore must return empty, not error"
        );
    }

    #[tokio::test]
    async fn chain_page_straddling_the_floor_returns_the_in_memory_portion() {
        // A page spanning both evicted and in-memory blocks must still correctly
        // return the in-memory portion — the Firestore portion legitimately can't be
        // fetched without live credentials in this test, but that must not corrupt or
        // drop what's actually available in RAM.
        let probe = Probe::spawn();
        let validators = vec![Validator::new(probe.id(), probe.pubkey_hex())];
        let mut node = Node::new(probe, validators);
        for i in 0..50u64 {
            node.try_produce(i, 1_000 + i);
        }
        node.chain = node.chain.clone().bound_to_window(10); // floor at 40, blocks 40..50 resident

        let resp = app(AppState::new(node))
            .oneshot(
                Request::builder()
                    .uri("/api/chain?offset=35&limit=15") // 35..50 straddles the floor at 40
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
            10,
            "only the in-memory portion is retrievable here"
        );
        assert_eq!(blocks[0].index, 40);
        assert_eq!(blocks.last().unwrap().index, 49);
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
