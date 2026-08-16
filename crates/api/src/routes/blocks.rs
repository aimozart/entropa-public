//! `GET /block/:index` — a single block's own static page.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse},
};

use crate::html::{esc, BLOCK_PAGE_HEAD};
use crate::AppState;

/// A single block's own static page — server-rendered HTML, no client JS at all.
/// Deliberately outside the explorer's live-polling list so it can never get
/// clobbered by a new block arriving while a visitor is reading it.
pub(crate) async fn block_page(
    State(s): State<AppState>,
    Path(index): Path<usize>,
) -> axum::response::Response {
    let n = s.node.lock().unwrap();
    let Some(block) = n.chain.blocks.get(index) else {
        return (
            StatusCode::NOT_FOUND,
            Html(format!(
                "{BLOCK_PAGE_HEAD}<body><div class=\"wrap\"><a class=\"back\" href=\"/\">← Back to explorer</a>\
                 <h1>Block {index} not found</h1><p>The chain doesn't have a block at that height (yet).</p>\
                 </div></body></html>"
            )),
        )
            .into_response();
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
    let has_next = n.chain.blocks.get(next).is_some();

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
    use tower::ServiceExt;

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
