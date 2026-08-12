//! # Entropa API
//!
//! An [axum](https://docs.rs/axum) gateway that serves the chain as JSON and hosts the
//! **Scryon** explorer page. Build the [`Router`] with [`app`] over an [`AppState`]
//! wrapping a node.
//!
//! Routes:
//! - `GET  /`            — the Scryon explorer (static HTML, works with JS off)
//! - `GET  /assets/*`     — the aimozart / Entropa mark, embedded favicons
//! - `GET  /api/health`  — network + consensus + signature scheme + height
//! - `GET  /api/chain`   — a page of blocks as JSON (`?limit=`, `?offset=`; defaults to the
//!   most recent 1000 — the full chain is too large for a single response once the chain
//!   grows past a few thousand blocks, so walk it with `offset` for anything older)
//! - `GET  /api/head`    — the head block
//! - `POST /api/tx`      — submit a transaction into the mempool
//! - `GET  /block/:index` — a single block's own static page (JS off safe, never
//!   disturbed by the explorer's live refresh)
//! - `GET  /flow`         — a readable feed of the Probe's decisions
//! - `GET  /robots.txt`, `/sitemap.xml`, `/llms.txt` — crawler/AI-result discoverability
//!
//! `POST /api/tx` requires a bearer API key when `AppState.api_keys` is non-empty (see
//! `AppState::with_api_keys`) — an authenticated submission's `from` field is overridden
//! with the key's owning partner name, so it can't be spoofed. When no keys are
//! configured, the endpoint stays open (the public open-core demo's default).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::{error_handling::HandleErrorLayer, BoxError};
use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use tower::{buffer::BufferLayer, limit::RateLimitLayer, ServiceBuilder};

use entropa_core::{Block, Transaction};
use entropa_node::Node;

/// Shared application state: the node whose chain we serve.
#[derive(Clone)]
pub struct AppState {
    pub node: Arc<Mutex<Node>>,
    /// Bearer key → owning partner name. Empty means `/api/tx` is open (dev/demo mode).
    pub api_keys: Arc<HashMap<String, String>>,
}

impl AppState {
    pub fn new(node: Node) -> Self {
        Self {
            node: Arc::new(Mutex::new(node)),
            api_keys: Arc::new(HashMap::new()),
        }
    }

    /// Parse `ENTROPA_API_KEYS`-style config: `"name1:key1,name2:key2"`.
    pub fn with_api_keys(mut self, spec: &str) -> Self {
        let keys = spec
            .split(',')
            .filter_map(|pair| {
                let (name, key) = pair.split_once(':')?;
                let (name, key) = (name.trim(), key.trim());
                (!name.is_empty() && !key.is_empty()).then(|| (key.to_string(), name.to_string()))
            })
            .collect();
        self.api_keys = Arc::new(keys);
        self
    }
}

/// Build the Entropa router — the Scryon page at `/` and the JSON API under `/api`.
pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/", get(scryon))
        .route(
            "/assets/favicon.ico",
            get(|| asset("image/x-icon", FAVICON_ICO)),
        )
        .route(
            "/assets/favicon-16.png",
            get(|| asset("image/png", FAVICON_16)),
        )
        .route(
            "/assets/favicon-32.png",
            get(|| asset("image/png", FAVICON_32)),
        )
        .route(
            "/assets/favicon-48.png",
            get(|| asset("image/png", FAVICON_48)),
        )
        .route(
            "/assets/favicon-180.png",
            get(|| asset("image/png", FAVICON_180)),
        )
        .route(
            "/assets/favicon-512.png",
            get(|| asset("image/png", FAVICON_512)),
        )
        .route("/block/{index}", get(block_page))
        .route("/flow", get(|| async { Html(FLOW_HTML) }))
        .route("/hire", get(|| async { Html(HIRE_HTML) }))
        .route(
            "/robots.txt",
            get(|| async { ([(header::CONTENT_TYPE, "text/plain")], ROBOTS_TXT) }),
        )
        .route(
            "/sitemap.xml",
            get(|| async { ([(header::CONTENT_TYPE, "application/xml")], SITEMAP_XML) }),
        )
        .route(
            "/llms.txt",
            get(|| async { ([(header::CONTENT_TYPE, "text/plain")], LLMS_TXT) }),
        )
        .route("/api/health", get(health))
        .route("/api/chain", get(chain))
        .route("/api/head", get(head))
        .route(
            "/api/tx",
            // Global cap on the write path — 20 req/s. Not per-key fairness yet (one
            // noisy caller can still crowd others out), but it stops a flood/bug from
            // hammering the mempool or GCS. Tighten to per-key limits once there's
            // real multi-partner traffic to justify the extra complexity.
            post(submit_tx).layer(
                ServiceBuilder::new()
                    .layer(HandleErrorLayer::new(rate_limit_error))
                    .layer(BufferLayer::new(1024))
                    .layer(RateLimitLayer::new(20, Duration::from_secs(1))),
            ),
        )
        .with_state(state)
}

// Two faces, one binary: `entropa.space` gets the marketing landing page,
// `scryon.entropa.space` (and anything else, e.g. the raw *.run.app URL) gets the
// real searchable block explorer. Both are embedded at compile time.
static LANDING_HTML: &str = include_str!("../scryon/landing.html");
static EXPLORER_HTML: &str = include_str!("../scryon/explorer.html");
static FLOW_HTML: &str = include_str!("../scryon/flow.html");
static HIRE_HTML: &str = include_str!("../scryon/hire.html");
static ROBOTS_TXT: &str = include_str!("../scryon/robots.txt");
static SITEMAP_XML: &str = include_str!("../scryon/sitemap.xml");
static LLMS_TXT: &str = include_str!("../scryon/llms.txt");

/// Serve the landing page on the root domain, the explorer everywhere else
/// (subdomain, raw Cloud Run URL, localhost during dev).
async fn scryon(headers: HeaderMap) -> Html<&'static str> {
    let host = headers
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    let is_root = host == "entropa.space" || host == "www.entropa.space";
    Html(if is_root { LANDING_HTML } else { EXPLORER_HTML })
}

// The aimozart / Entropa mark, embedded at compile time so the binary is self-contained.
static FAVICON_ICO: &[u8] = include_bytes!("../scryon/assets/favicon.ico");
static FAVICON_16: &[u8] = include_bytes!("../scryon/assets/favicon-16.png");
static FAVICON_32: &[u8] = include_bytes!("../scryon/assets/favicon-32.png");
static FAVICON_48: &[u8] = include_bytes!("../scryon/assets/favicon-48.png");
static FAVICON_180: &[u8] = include_bytes!("../scryon/assets/favicon-180.png");
static FAVICON_512: &[u8] = include_bytes!("../scryon/assets/favicon-512.png");

async fn asset(content_type: &'static str, bytes: &'static [u8]) -> impl IntoResponse {
    ([(header::CONTENT_TYPE, content_type)], bytes)
}

async fn health(State(s): State<AppState>) -> Json<serde_json::Value> {
    let n = s.node.lock().unwrap();
    Json(serde_json::json!({
        "status": "ok",
        "network": "Entropa",
        "tagline": "Entropa is boring. All we do is keep your AI agents auditable.",
        "consensus": entropa_node::consensus::NAME,
        "signature": "ML-DSA (NIST FIPS-204)",
        "height": n.height(),
    }))
}

/// `?limit=` (default 1000, capped at 2000) and `?offset=` (default: the tail of the
/// chain, i.e. the most recent blocks) — the full chain is too large for one Cloud Run
/// response once it grows past a few thousand blocks; walk it with `offset` for history.
#[derive(serde::Deserialize)]
struct ChainQuery {
    limit: Option<usize>,
    offset: Option<usize>,
}

async fn chain(State(s): State<AppState>, Query(q): Query<ChainQuery>) -> Json<Vec<Block>> {
    let n = s.node.lock().unwrap();
    let total = n.chain.blocks.len();
    let limit = q.limit.unwrap_or(1000).min(2000);
    let offset = q.offset.unwrap_or_else(|| total.saturating_sub(limit));
    let end = (offset + limit).min(total);
    let page = n.chain.blocks.get(offset.min(total)..end).unwrap_or(&[]);
    Json(page.to_vec())
}

async fn head(State(s): State<AppState>) -> Json<Option<Block>> {
    let n = s.node.lock().unwrap();
    Json(n.chain.head().cloned())
}

/// A single block's own static page — server-rendered HTML, no client JS at all.
/// Deliberately outside the explorer's live-polling list so it can never get
/// clobbered by a new block arriving while a visitor is reading it.
async fn block_page(
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

/// Minimal HTML-escaping for any field that could carry user-submitted content
/// (transactions come in via `POST /api/tx`).
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

const BLOCK_PAGE_HEAD: &str = r#"<!doctype html><html><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Entropa — block detail</title>
<link rel="icon" href="/assets/favicon.ico">
<style>
:root{color-scheme:dark}
body{margin:0;background:#0a0e14;color:#dbe4f0;font:15px/1.5 -apple-system,system-ui,sans-serif}
.wrap{max-width:760px;margin:0 auto;padding:32px 24px 64px}
.back{color:#7fb8ff;text-decoration:none;font-size:14px}
.back:hover{text-decoration:underline}
h1{margin:16px 0 20px;font-size:28px}
h2{margin:32px 0 12px;font-size:18px;color:#9fb3cc}
table{width:100%;border-collapse:collapse;background:rgba(255,255,255,.03);border:1px solid rgba(255,255,255,.08);border-radius:10px;overflow:hidden}
.fields th,.fields td{padding:10px 14px;text-align:left;border-top:1px solid rgba(255,255,255,.06)}
.fields tr:first-child th,.fields tr:first-child td{border-top:none}
.fields th{color:#8ea0b8;font-weight:500;width:160px}
.txs th,.txs td{padding:8px 14px;text-align:left;border-top:1px solid rgba(255,255,255,.06);font-size:13px}
.txs th{color:#8ea0b8;font-weight:500}
.mono{font-family:ui-monospace,SFMono-Regular,Menlo,monospace;font-size:13px;word-break:break-all;color:#a8d8ff}
.nav{margin-top:28px;display:flex;justify-content:space-between;font-size:14px}
.nav a{color:#7fb8ff;text-decoration:none}
.nav a:hover{text-decoration:underline}
.muted{color:#5a6b80}
.sep{display:none}
</style></head>
"#;

/// Turns a buffered/rate-limited request's failure (queue full, or over the rate cap)
/// into a proper HTTP response instead of the connection just dying.
async fn rate_limit_error(err: BoxError) -> (StatusCode, String) {
    (
        StatusCode::TOO_MANY_REQUESTS,
        format!("rate limited: {err}"),
    )
}

/// Pull the bearer token out of `Authorization: Bearer <key>`, if present.
fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
}

async fn submit_tx(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(mut tx): Json<Transaction>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !s.api_keys.is_empty() {
        let partner = bearer_token(&headers)
            .and_then(|token| s.api_keys.get(token))
            .ok_or(StatusCode::UNAUTHORIZED)?;
        // The authenticated key's owner overrides whatever `from` the client sent —
        // a partner can't submit a transaction claiming to be someone else.
        tx.from = partner.clone();
    }
    let mut n = s.node.lock().unwrap();
    n.submit(tx);
    Ok(Json(
        serde_json::json!({ "accepted": true, "pending": n.mempool.len() }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use entropa_core::Probe;
    use entropa_node::Validator;
    use tower::ServiceExt;

    fn node_with_one_block() -> Node {
        let probe = Probe::spawn();
        let validators = vec![Validator::new(probe.id(), probe.pubkey_hex())];
        let mut node = Node::new(probe, validators);
        node.submit(Transaction::new("boot", "genesis", "big bang"));
        node.try_produce(0, 1_000);
        node
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

    #[tokio::test]
    async fn root_serves_scryon() {
        let resp = app(AppState::new(node_with_one_block()))
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
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

    #[tokio::test]
    async fn crawler_files_are_served() {
        for path in ["/robots.txt", "/sitemap.xml", "/llms.txt"] {
            let resp = app(AppState::new(node_with_one_block()))
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::OK, "{path} should return 200");
        }
    }

    #[tokio::test]
    async fn hire_page_served() {
        let resp = app(AppState::new(node_with_one_block()))
            .oneshot(Request::builder().uri("/hire").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
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
