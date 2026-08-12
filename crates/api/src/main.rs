//! Entropa demo node — runs a live network you can watch in the Scryon explorer.
//!
//! This public build's demo proposer is a small deterministic stand-in — the real
//! Entropa network runs AI Probes (Claude-driven decision-making) via a private
//! `entropa-agents` crate; see OPEN_SOURCE.md for the open-core split. What you see
//! here is the real thing: genuine ML-DSA (FIPS-204) signatures and genuine
//! Proof-of-Entropy consensus, every round. Open http://localhost:8080 to watch the
//! constellation grow.

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use entropa_api::{app, AppState};
use entropa_core::{Probe, Transaction};
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
        format!("round {round} · {pending} pending · cosmic beacon verified"),
    )
}

#[tokio::main]
async fn main() {
    // One validator node — the sole proposer in this demo network.
    let probe = Probe::spawn();
    let validators = vec![Validator::new(probe.id(), probe.pubkey_hex())];
    let state = AppState::new(Node::new(probe, validators));

    // Background: each round, decide → submit → produce a real Proof-of-Entropy block.
    let ticker = Arc::clone(&state.node);
    tokio::spawn(async move {
        let mut round: u64 = 0;
        loop {
            {
                let mut n = ticker.lock().unwrap();
                let tx = demo_decision(round, n.mempool.len());
                n.submit(tx);
                let _ = n.try_produce(round, now());
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
