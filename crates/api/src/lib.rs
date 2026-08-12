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
//! - `GET  /api/chain`   — every block as JSON
//! - `GET  /api/head`    — the head block
//! - `POST /api/tx`      — submit a transaction into the mempool

use std::sync::{Arc, Mutex};

use axum::{
    extract::State,
    http::header,
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};

use entropa_core::{Block, Transaction};
use entropa_node::Node;

/// Shared application state: the node whose chain we serve.
#[derive(Clone)]
pub struct AppState {
    pub node: Arc<Mutex<Node>>,
}

impl AppState {
    pub fn new(node: Node) -> Self {
        Self {
            node: Arc::new(Mutex::new(node)),
        }
    }
}

/// Build the Entropa router — the Scryon page at `/` and the JSON API under `/api`.
pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/", get(scryon))
        .route("/assets/favicon.ico", get(|| asset("image/x-icon", FAVICON_ICO)))
        .route("/assets/favicon-16.png", get(|| asset("image/png", FAVICON_16)))
        .route("/assets/favicon-32.png", get(|| asset("image/png", FAVICON_32)))
        .route("/assets/favicon-48.png", get(|| asset("image/png", FAVICON_48)))
        .route("/assets/favicon-180.png", get(|| asset("image/png", FAVICON_180)))
        .route("/assets/favicon-512.png", get(|| asset("image/png", FAVICON_512)))
        .route("/api/health", get(health))
        .route("/api/chain", get(chain))
        .route("/api/head", get(head))
        .route("/api/tx", post(submit_tx))
        .with_state(state)
}

/// The Scryon explorer — static HTML, embedded at compile time.
async fn scryon() -> Html<&'static str> {
    Html(include_str!("../scryon/index.html"))
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
        "tagline": "Entropa is boring. All we do is keep you safe.",
        "consensus": entropa_node::consensus::NAME,
        "signature": "ML-DSA (NIST FIPS-204)",
        "height": n.height(),
    }))
}

async fn chain(State(s): State<AppState>) -> Json<Vec<Block>> {
    let n = s.node.lock().unwrap();
    Json(n.chain.blocks.clone())
}

async fn head(State(s): State<AppState>) -> Json<Option<Block>> {
    let n = s.node.lock().unwrap();
    Json(n.chain.head().cloned())
}

async fn submit_tx(
    State(s): State<AppState>,
    Json(tx): Json<Transaction>,
) -> Json<serde_json::Value> {
    let mut n = s.node.lock().unwrap();
    n.submit(tx);
    Json(serde_json::json!({ "accepted": true, "pending": n.mempool.len() }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
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
            .oneshot(Request::builder().uri("/api/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn chain_returns_ok() {
        let resp = app(AppState::new(node_with_one_block()))
            .oneshot(Request::builder().uri("/api/chain").body(Body::empty()).unwrap())
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
}
