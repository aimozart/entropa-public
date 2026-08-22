//! `GET /dashboard/:partner` — a public, per-customer attestation dashboard.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse},
};

use entropa_core::Block;

use crate::html::{esc, BLOCK_PAGE_HEAD};
use crate::{persistence, AppState};

/// Every transaction submitted by `partner` (matched against `Transaction.from`), most
/// recent block first, rendered as a public dashboard page. `None` if the partner has
/// no attestations at all — callers decide how to 404 without needing to know
/// anything about chain internals.
///
/// Takes `blocks` directly (oldest-first, same convention as `Chain::blocks`) rather
/// than `AppState` or `Node` — no dependency on *how* the partner name arrived (a
/// path segment today; a subdomain's `Host` header once customer subdomains exist)
/// or on *where* the blocks came from (the in-memory window, a Firestore fallback
/// scan for evicted history, or both concatenated), so a future subdomain handler
/// can call this completely unchanged and this logic stays unit-testable without I/O.
///
/// `billed` is whether this partner has a Stripe customer wired up
/// (`AppState.stripe_customers`). A `false` value renders a visible demo/example
/// banner directly on the page — not just on the landing-page link that points here,
/// so anyone reaching this URL directly (bookmarked, shared, crawled) still sees it's
/// not a real customer. Closes a real gap found via a Paxel growth-area review
/// (2026-08-18): the "example customer" wording only lived in `landing.html`'s link
/// text, never on the dashboard page itself.
pub(crate) fn render_partner_dashboard(
    blocks: &[Block],
    partner: &str,
    billed: bool,
) -> Option<String> {
    let rows: String = blocks
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

    let demo_banner = if billed {
        String::new()
    } else {
        "<p class=\"muted\" style=\"border:1px solid #7c5cff;border-radius:8px;padding:10px 14px;\">\
         🔍 This is an <strong>example customer dashboard</strong> for demo/marketing purposes — \
         not a real, billed customer of Entropa.</p>"
            .to_string()
    };

    Some(format!(
        "{BLOCK_PAGE_HEAD}<body><div class=\"wrap\">\
        <a class=\"back\" href=\"/\">← Back to explorer</a>\
        <h1>{partner_esc}'s Attestation Dashboard</h1>\
        {demo_banner}\
        <p class=\"muted\">Every action recorded on Entropa by this customer, with a link to \
        its independently-verifiable Attestation Receipt.</p>\
        <table class=\"txs\"><tr><th>Block</th><th>Kind</th><th>Payload</th><th>Receipt</th></tr>{rows}</table>\
        </div></body></html>",
        partner_esc = esc(partner),
    ))
}

/// Scan every evicted block (index `0..base_index`, i.e. everything `bound_to_window`
/// has trimmed out of RAM) for transactions from `partner`, since there's no
/// per-partner index — deliberate (Captain's call, 2026-08-22): dashboard traffic is
/// low enough that a second index isn't worth the added write-path complexity yet.
/// Unbounded on purpose for now — cost grows with total evicted history, which is
/// fine while only the tiny demo partner exists; revisit (a real per-partner index,
/// or a capped lookback) once a partner has genuine long-lived volume. Fails soft to
/// an empty `Vec` if Firestore/the metadata server is unreachable, same posture as
/// every other fallback in this module.
async fn scan_evicted_history_for_partner(
    epoch: &str,
    base_index: u64,
    partner: &str,
) -> Vec<Block> {
    let mut found = Vec::new();
    for idx in 0..base_index {
        if let Some(block) = persistence::load_block_in_lineage(epoch, idx).await {
            if block.transactions.iter().any(|tx| tx.from == partner) {
                found.push(block);
            }
        }
    }
    found
}

pub(crate) async fn dashboard_page(
    State(s): State<AppState>,
    Path(partner): Path<String>,
) -> axum::response::Response {
    let (mem_blocks, base_index, epoch) = {
        let n = s.node.lock().unwrap();
        (
            n.chain.blocks.clone(),
            n.chain.base_index,
            s.active_epoch.clone(),
        )
    };
    let billed = s.stripe_customers.contains_key(&partner);

    let mut all_blocks = match epoch {
        Some(epoch) if base_index > 0 => {
            scan_evicted_history_for_partner(&epoch, base_index, &partner).await
        }
        _ => Vec::new(),
    };
    all_blocks.extend(mem_blocks);

    match render_partner_dashboard(&all_blocks, &partner, billed) {
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
    async fn unbilled_partner_dashboard_shows_demo_banner() {
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
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(
            html.contains("example customer dashboard"),
            "an unbilled partner's dashboard must self-label as a demo, not just rely on \
             whatever page linked to it"
        );
        assert!(!html.contains("real, billed customer of Entropa</strong>"));
    }

    #[tokio::test]
    async fn billed_partner_dashboard_has_no_demo_banner() {
        let state = AppState::new(node_with_partner_tx("acme", "attest", "real payload"))
            .with_stripe_customers("acme:cus_123");
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/dashboard/acme")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(
            !html.contains("example customer dashboard"),
            "a real, billed customer must never be shown a demo banner"
        );
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

    #[tokio::test]
    async fn scan_evicted_history_for_partner_fails_soft_without_gcp_project() {
        std::env::remove_var("GCP_PROJECT");
        let found =
            super::scan_evicted_history_for_partner("some-epoch", 50, "john-q-public").await;
        assert!(
            found.is_empty(),
            "must fail soft to empty, not error, when Firestore is unreachable"
        );
    }

    #[tokio::test]
    async fn dashboard_still_shows_in_memory_attestations_when_evicted_history_is_unreachable() {
        // A partner with attestations both before and after eviction: the evicted one
        // legitimately can't be recovered without live GCP credentials in this test
        // (same limitation as the other fallback tests), but the still-in-memory one
        // must not be lost or corrupted by the attempted (and failed) fallback scan.
        std::env::remove_var("GCP_PROJECT");
        let probe = Probe::spawn();
        let validators = vec![Validator::new(probe.id(), probe.pubkey_hex())];
        let mut node = Node::new(probe, validators);
        node.submit(Transaction::new("john-q-public", "attest", "old-evicted"));
        node.try_produce(0, 1_000);
        node.submit(Transaction::new("john-q-public", "attest", "recent"));
        node.try_produce(1, 2_000);
        node.chain = node.chain.clone().bound_to_window(1);
        assert_eq!(
            node.chain.base_index, 1,
            "test setup: block 0 must be evicted"
        );

        let resp = app(AppState::new(node).with_active_epoch("test-epoch"))
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
        assert!(
            html.contains("recent"),
            "the still-in-memory attestation must still appear even though the evicted-history \
             scan came back empty"
        );
    }
}
