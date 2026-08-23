//! `GET /block/:index` — a single block's own static page.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse},
};

use crate::html::{esc, BLOCK_PAGE_HEAD};
use crate::{persistence, AppState};

fn not_found_response(index: usize) -> axum::response::Response {
    (
        StatusCode::NOT_FOUND,
        Html(format!(
            "{BLOCK_PAGE_HEAD}<body><div class=\"wrap\"><a class=\"back\" href=\"/\">← Back to explorer</a>\
             <h1>Block {index} not found</h1><p>The chain doesn't have a block at that height (yet).</p>\
             </div></body></html>"
        )),
    )
        .into_response()
}

/// A single block's own static page — server-rendered HTML, no client JS at all.
/// Deliberately outside the explorer's live-polling list so it can never get
/// clobbered by a new block arriving while a visitor is reading it.
///
/// `chain.blocks` only holds the bounded in-memory window (`Chain::bound_to_window`)
/// — its Vec positions are *relative* to `chain.base_index`, not the block's absolute
/// chain index, so `index` must be translated before indexing into it. Anything below
/// `base_index` has been evicted from RAM and is fetched from Firestore instead, same
/// fallback pattern as `/api/chain` and `/api/receipt/:id`.
pub(crate) async fn block_page(
    State(s): State<AppState>,
    Path(index): Path<usize>,
) -> axum::response::Response {
    let (block, height, epoch) = {
        let n = s.node.lock().unwrap();
        let base_index = n.chain.base_index as usize;
        let block = index
            .checked_sub(base_index)
            .and_then(|rel| n.chain.blocks.get(rel))
            .cloned();
        (block, n.chain.height() as usize, s.active_epoch.clone())
    };

    let block = match block {
        Some(block) => block,
        None => {
            let Some(epoch) = epoch else {
                return not_found_response(index);
            };
            match persistence::load_block_in_lineage(&epoch, index as u64).await {
                Some(block) => block,
                None => return not_found_response(index),
            }
        }
    };

    let tx_rows: String = block
        .transactions
        .iter()
        .map(|tx| {
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
                esc(&tx.from),
                esc(&tx.kind),
                esc(&tx.payload),
            )
        })
        .collect();

    let prev = index.checked_sub(1);
    let next = index + 1;
    let has_next = next < height;

    let nav = format!(
        "{}<span class=\"sep\">·</span>{}",
        prev.map(|p| format!("<a href=\"/block/{p}\">← block {p}</a>"))
            .unwrap_or_else(|| "<span class=\"muted\">← genesis</span>".to_string()),
        if has_next {
            format!("<a href=\"/block/{next}\">block {next} →</a>")
        } else {
            "<span class=\"muted\">latest →</span>".to_string()
        }
    );

    let html = format!(
        "{BLOCK_PAGE_HEAD}<body><div class=\"wrap\">\
        <a class=\"back\" href=\"/\">← Back to explorer</a>\
        <h1>Block {index}</h1>\
        <table class=\"fields\">\
        <tr><th>Timestamp</th><td>{ts}</td></tr>\
        <tr><th>Prev hash</th><td class=\"mono\">{prev_hash}</td></tr>\
        <tr><th>Beacon</th><td class=\"mono\">{beacon}</td></tr>\
        <tr><th>Proposer</th><td class=\"mono\">{proposer}</td></tr>\
        <tr><th>Proposer pubkey</th><td class=\"mono\">{pubkey}</td></tr>\
        <tr><th>Hash</th><td class=\"mono\">{hash}</td></tr>\
        <tr><th>Signature</th><td class=\"mono\">{sig}</td></tr>\
        </table>\
        <h2>{tx_count} transaction{tx_plural}</h2>\
        <table class=\"txs\"><tr><th>From</th><th>Kind</th><th>Payload</th></tr>{tx_rows}</table>\
        <div class=\"nav\">{nav}</div>\
        </div></body></html>",
        ts = block.timestamp,
        prev_hash = esc(&block.prev_hash),
        beacon = esc(&block.beacon),
        proposer = esc(&block.proposer_id),
        pubkey = esc(&block.proposer_pubkey),
        hash = esc(&block.hash),
        sig = esc(&block.signature),
        tx_count = block.transactions.len(),
        tx_plural = if block.transactions.len() == 1 {
            ""
        } else {
            "s"
        },
    );

    Html(html).into_response()
}

#[cfg(test)]
mod tests {
    use crate::test_support::node_with_one_block;
    use crate::{app, AppState};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use entropa_core::Probe;
    use entropa_node::{Node, Validator};
    use tower::ServiceExt;

    /// Regression test for the 2026-08-23 incident (found in the private repo,
    /// applies identically here since both share `bound_to_window`): after eviction,
    /// `chain.blocks` is a Vec of only the retained window at *relative* offsets —
    /// but `block_page` was looking blocks up by their raw *absolute* chain index,
    /// which is only correct before the first eviction. This builds a small chain,
    /// evicts down to a 10-block window (floor at height 40), and requests a block
    /// that's absolutely within the window but not at that position in the trimmed Vec.
    #[tokio::test]
    async fn block_page_finds_block_still_within_the_window_after_eviction() {
        let probe = Probe::spawn();
        let validators = vec![Validator::new(probe.id(), probe.pubkey_hex())];
        let mut node = Node::new(probe, validators);
        for i in 0..50u64 {
            node.try_produce(i, 1_000 + i);
        }
        node.chain = node.chain.clone().bound_to_window(10);
        assert_eq!(node.chain.base_index, 40, "test setup: floor must be 40");

        let resp = app(AppState::new(node))
            .oneshot(
                Request::builder()
                    .uri("/block/45")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "block 45 is within the retained window (40..49) and must be found"
        );
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(
            html.contains("Block 45"),
            "page must render the right block"
        );
    }

    #[tokio::test]
    async fn block_page_found() {
        let resp = app(AppState::new(node_with_one_block()))
            .oneshot(
                Request::builder()
                    .uri("/block/0")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn block_page_missing_is_404() {
        let resp = app(AppState::new(node_with_one_block()))
            .oneshot(
                Request::builder()
                    .uri("/block/99")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
