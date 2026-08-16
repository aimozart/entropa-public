//! `GET /dashboard/:partner` — a public, per-customer attestation dashboard.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse},
};

use entropa_node::Node;

use crate::html::{esc, BLOCK_PAGE_HEAD};
use crate::AppState;

/// Every transaction submitted by `partner` (matched against `Transaction.from`), most
/// recent block first, rendered as a public dashboard page. `None` if the partner has
/// no attestations at all — callers decide how to 404 without needing to know
/// anything about chain internals.
///
/// Deliberately takes `&Node` rather than `AppState` and returns a plain `String` —
/// no dependency on *how* the partner name arrived (a path segment today; a
/// subdomain's `Host` header once customer subdomains exist), so a future subdomain
/// handler can call this completely unchanged.
pub(crate) fn render_partner_dashboard(n: &Node, partner: &str) -> Option<String> {
    let rows: String = n
        .chain
        .blocks
        .iter()
        .rev()
        .flat_map(|block| {
            block
                .transactions
                .iter()
                .filter(move |tx| tx.from == partner)
                .map(move |tx| (block.index, tx))
        })
        .map(|(index, tx)| {
            format!(
                "<tr><td><a href=\"/block/{index}\">{index}</a></td><td>{kind}</td>\
                 <td>{payload}</td><td><a href=\"/api/receipt/{receipt_id}\">view receipt →</a></td></tr>",
                kind = esc(&tx.kind),
                payload = esc(&tx.payload),
                receipt_id = tx.content_hash(),
            )
        })
        .collect();

    if rows.is_empty() {
        return None;
    }

    Some(format!(
        "{BLOCK_PAGE_HEAD}<body><div class=\"wrap\">\
        <a class=\"back\" href=\"/\">← Back to explorer</a>\
        <h1>{partner_esc}'s Attestation Dashboard</h1>\
        <p class=\"muted\">Every action recorded on Entropa by this customer, with a link to \
        its independently-verifiable Attestation Receipt.</p>\
        <table class=\"txs\"><tr><th>Block</th><th>Kind</th><th>Payload</th><th>Receipt</th></tr>{rows}</table>\
        </div></body></html>",
        partner_esc = esc(partner),
    ))
}

pub(crate) async fn dashboard_page(
    State(s): State<AppState>,
    Path(partner): Path<String>,
) -> axum::response::Response {
    let n = s.node.lock().unwrap();
    match render_partner_dashboard(&n, &partner) {
        Some(html) => Html(html).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Html(format!(
                "{BLOCK_PAGE_HEAD}<body><div class=\"wrap\">\
                 <a class=\"back\" href=\"/\">← Back to explorer</a>\
                 <h1>No dashboard found at this address</h1></div></body></html>"
            )),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::{node_with_one_block, node_with_partner_tx};
    use crate::{app, AppState};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use entropa_core::{Probe, Transaction};
    use entropa_node::{Node, Validator};
    use tower::ServiceExt;

    #[tokio::test]
    async fn dashboard_shows_partner_attestations() {
        let resp = app(AppState::new(node_with_partner_tx(
            "john-q-public",
            "attest",
            "demo payload",
        )))
        .oneshot(
            Request::builder()
                .uri("/dashboard/john-q-public")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(html.contains("attest"));
        assert!(html.contains("demo payload"));
        assert!(html.contains("/api/receipt/"));
    }

    #[tokio::test]
    async fn dashboard_404s_for_unknown_partner() {
        let resp = app(AppState::new(node_with_one_block()))
            .oneshot(
                Request::builder()
                    .uri("/dashboard/nobody-here")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn dashboard_escapes_untrusted_fields() {
        let resp = app(AppState::new(node_with_partner_tx(
            "john-q-public",
            "attest",
            "<script>alert(1)</script>",
        )))
        .oneshot(
            Request::builder()
                .uri("/dashboard/john-q-public")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[tokio::test]
    async fn dashboard_orders_most_recent_first() {
        let probe = Probe::spawn();
        let validators = vec![Validator::new(probe.id(), probe.pubkey_hex())];
        let mut node = Node::new(probe, validators);
        node.submit(Transaction::new("john-q-public", "attest", "first"));
        node.try_produce(0, 1_000);
        node.submit(Transaction::new("john-q-public", "attest", "second"));
        node.try_produce(1, 2_000);

        let resp = app(AppState::new(node))
            .oneshot(
                Request::builder()
                    .uri("/dashboard/john-q-public")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let html = String::from_utf8(body.to_vec()).unwrap();
        // Search for the payload inside its table cell specifically — the page's own CSS
        // contains the literal substring "first" (from a `:first-child` selector) earlier
        // in the document, which would give a false match on a loose `html.find("first")`.
        let pos_first = html.find("<td>first</td>").unwrap();
        let pos_second = html.find("<td>second</td>").unwrap();
        assert!(
            pos_second < pos_first,
            "most recent block (\"second\") must appear before older block (\"first\")"
        );
    }
}
