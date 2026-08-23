//! # Entropa API
//!
//! An [axum](https://docs.rs/axum) gateway that serves the chain as JSON and hosts the
//! **Scryon** explorer page. Build the [`Router`] with [`app`] over an [`AppState`]
//! wrapping a node.
//!
//! Routes:
//! - `GET  /`            — the Scryon explorer (static HTML, works with JS off)
//! - `GET  /assets/*`     — the aimozart / Entropa mark, embedded favicons
//! - `GET  /api/health`, `GET /healthz` — network + consensus + signature scheme + height
//!   (`/healthz` is an alias for the same handler, for tooling that expects that path)
//! - `GET  /api/chain`   — a page of blocks as JSON (`?limit=`, `?offset=`; defaults to the
//!   most recent 1000 — the full chain is too large for a single response once the chain
//!   grows past a few thousand blocks, so walk it with `offset` for anything older)
//! - `GET  /api/head`    — the head block
//! - `GET  /api/receipt/:id` — the Attestation Receipt for a transaction (looked up by the
//!   `receipt_id` returned from `POST /api/tx`) — plain-English explanation plus full
//!   technical proof, independently re-verified on every request
//! - `POST /api/tx`      — submit a transaction into the mempool
//! - `GET  /block/:index` — a single block's own static page (JS off safe, never
//!   disturbed by the explorer's live refresh)
//! - `GET  /flow`         — a readable feed of the Probe's decisions
//! - `GET  /glossary`     — plain-English definitions of every term this product uses
//! - `GET  /robots.txt`, `/sitemap.xml`, `/llms.txt` — crawler/AI-result discoverability
//!
//! `POST /api/tx` requires a bearer API key when `AppState.api_keys` is non-empty (see
//! `AppState::with_api_keys`) — an authenticated submission's `from` field is overridden
//! with the key's owning partner name, so it can't be spoofed. When no keys are
//! configured, the endpoint stays open (the public open-core demo's default).

use std::time::Duration;

use axum::error_handling::HandleErrorLayer;
use axum::{
    http::{header, HeaderMap},
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use tower::{buffer::BufferLayer, limit::RateLimitLayer, ServiceBuilder};

mod html;
pub mod persistence;
mod routes;
mod state;
#[cfg(test)]
mod test_support;

pub use state::AppState;

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
        .route("/block/{index}", get(routes::blocks::block_page))
        .route("/flow", get(|| async { Html(FLOW_HTML) }))
        .route("/hire", get(|| async { Html(HIRE_HTML) }))
        .route("/resume", get(|| async { Html(RESUME_HTML) }))
        .route("/glossary", get(|| async { Html(GLOSSARY_HTML) }))
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
        .route("/api/health", get(routes::chain_data::health))
        // Alias for the /healthz convention (Kubernetes/Cloud Run health-check
        // scanners and tooling commonly look for this exact path) — same handler,
        // same response, just a second path to the same signal.
        .route("/healthz", get(routes::chain_data::health))
        .route("/api/chain", get(routes::chain_data::chain))
        .route("/api/head", get(routes::chain_data::head))
        .route("/api/receipt/{id}", get(routes::receipt::receipt))
        .route(
            "/api/tx",
            // Global cap on the write path — 20 req/s across everyone, a backstop against
            // the service itself getting hammered. Per-key fairness (one partner can't
            // starve another underneath this ceiling) is enforced separately inside
            // `submit_tx` via `AppState.rate_buckets`.
            post(routes::tx::submit_tx).layer(
                ServiceBuilder::new()
                    .layer(HandleErrorLayer::new(routes::tx::rate_limit_error))
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
static RESUME_HTML: &str = include_str!("../scryon/resume.html");
static GLOSSARY_HTML: &str = include_str!("../scryon/glossary.html");
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::node_with_one_block;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn root_serves_scryon() {
        let resp = app(AppState::new(node_with_one_block()))
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
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
    async fn glossary_page_served() {
        let resp = app(AppState::new(node_with_one_block()))
            .oneshot(
                Request::builder()
                    .uri("/glossary")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
