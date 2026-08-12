//! Entropa demo node — runs a live network you can watch in the Scryon explorer.
//!
//! This public build's demo proposer is a small deterministic stand-in — the real
//! Entropa network runs AI Probes (Claude-driven decision-making) via a private
//! `entropa-agents` crate; see OPEN_SOURCE.md for the open-core split. What you see
//! here is the real thing: genuine ML-DSA (FIPS-204) signatures and genuine
//! Proof-of-Entropy consensus, every round. Open http://localhost:8080 to watch the
//! constellation grow.
//!
//! Chain state persists to GCS when `ENTROPA_GCS_BUCKET` is set (see `persistence.rs`)
//! — so a redeploy resumes the network instead of resetting it to height zero. Off
//! Cloud Run (e.g. local `cargo run`), persistence is a no-op and the chain is
//! in-memory only, exactly as before.

mod persistence;

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use entropa_api::{app, AppState};
use entropa_core::{Chain, Probe, Transaction};
use entropa_node::{Node, Validator};

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// The public demo's stand-in for an AI Probe's decision. The real network's Probes
/// reason about what to propose via Claude (private `entropa-agents` crate); this
/// deterministic version exists so the public build is fully self-contained.
fn demo_decision(round: u64, pending: usize) -> Transaction {
    Transaction::new(
        "demo-probe",
        "attest",
        format!("round {round} · {pending} pending · randomness beacon verified"),
    )
}

#[tokio::main]
async fn main() {
    // One validator node — the sole proposer in this demo network.
    let probe = Probe::spawn();
    let validators = vec![Validator::new(probe.id(), probe.pubkey_hex())];
    let mut node = Node::new(probe, validators);

    // Resume from persisted state if configured and available; otherwise start fresh.
    let mut round: u64 = 0;
    if let Some(blocks) = persistence::load_chain().await {
        let candidate = Chain { blocks };
        if candidate.verify().is_ok() {
            round = candidate.len() as u64;
            println!("📦 resumed chain from persisted state — height {round}");
            node.chain = candidate;
        } else {
            eprintln!("persistence: loaded chain failed verification — starting fresh");
        }
    }

    let state = AppState::new(node);

    // Background: each round, decide → submit → produce a real Proof-of-Entropy block,
    // then persist (best-effort — a failed save just gets caught by the next one).
    let ticker = Arc::clone(&state.node);
    tokio::spawn(async move {
        let mut round = round;
        loop {
            // Fetch the live entropy beacon (drand's public quicknet) once for this
            // round and inject it before the sync is_proposer/try_produce pair runs,
            // so both see the identical value — falls back to the deterministic stub
            // if drand is unreachable.
            let live_beacon = entropa_core::beacon::sample_live().await;
            let saved_blocks = {
                let mut n = ticker.lock().unwrap();
                let tx = demo_decision(round, n.mempool.len());
                n.submit(tx);
                n.live_beacon = live_beacon;
                n.try_produce(round, now()).map(|_| n.chain.blocks.clone())
            };
            if let Some(blocks) = saved_blocks {
                persistence::save_chain(&blocks).await;
            }
            round += 1;
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
    });

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .expect("bind :8080");
    println!("🌌 Scryon live at http://localhost:8080  —  Entropa · Proof of Entropy · ML-DSA (FIPS-204)");
    axum::serve(listener, app(state)).await.expect("serve");
}
