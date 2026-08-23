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

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use entropa_api::persistence;
use entropa_api::{app, AppState};
use entropa_core::{Chain, Probe, Transaction};
use entropa_node::{Node, Validator};

/// How many of the most recent blocks stay in memory (`Chain::bound_to_window`) —
/// applied on resume and continuously after every produced block, so RAM usage
/// stays flat as height grows instead of growing without bound. Mirrors the private
/// build's identical constant/reasoning (`entropa-chain/crates/api/src/main.rs`) —
/// see `SESSION_STATE.md` § Bounded-memory chain architecture there for the full
/// incident history; this file isn't mirrored between the two repos, so the wiring
/// has to be applied here independently rather than copied.
const IN_MEMORY_WINDOW: usize = 20_000;

/// Same defensive pattern as the private build's `ROUND_WATCHDOG_SECS`/`TICK_LOG_SECS`
/// (`entropa-chain/crates/api/src/round.rs`) — a real incident there found a
/// validator's production loop could go completely silent (no crash, no log output)
/// while `/api/health` stayed green. This demo loop's per-iteration work is already
/// more tightly timeout-bounded (no peer-network/quorum calls), so the risk is lower,
/// but the same "silent forever" blind spot is possible in principle — applied here
/// independently since main.rs isn't mirrored between the two repos.
const ROUND_WATCHDOG_SECS: u64 = 90;
const TICK_LOG_SECS: u64 = 30;

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
    //
    // Storage is epoch-namespaced (`chains/{epoch_id}/blocks/{index}`) — see
    // persistence.rs's module docs for why: a flat, index-only key scheme let a
    // restarted chain silently overwrite a prior chain's history at the same
    // indices (the incident that motivated this design). Whenever a rollback is
    // detected — some loaded data failed to form a valid prefix, whether that's a
    // gap found within the current epoch or the one-time migration from the old
    // flat scheme — we rotate to a brand new epoch instead of continuing to write
    // into the same paths. The old epoch's remaining data is never touched again.
    let mut round: u64;
    let active_epoch: String = match persistence::get_current_epoch().await {
        Some(epoch_id) => {
            let (recovered, loaded_count) = match persistence::load_chain(&epoch_id).await {
                Some(blocks) => {
                    let loaded_count = blocks.len();
                    (Chain::recover_longest_valid_prefix(blocks), loaded_count)
                }
                None => (Chain::default(), 0),
            };
            round = recovered.len() as u64;

            if !entropa_core::rollback_detected(loaded_count, recovered.len()) {
                println!(
                    "📦 resumed chain from persisted state — epoch {epoch_id}, height {round}"
                );
                node.chain = recovered.bound_to_window(IN_MEMORY_WINDOW);
                epoch_id
            } else {
                // A gap was found within the *current* epoch. Recover what's valid,
                // then rotate to a new epoch so this epoch's un-recovered tail is
                // frozen in place forever rather than getting overwritten by
                // whatever this process produces next. The new epoch records a
                // parent pointer instead of copying every recovered block — see
                // persistence.rs's module doc for why (copying is O(n) with height
                // and risks exceeding a real deploy platform's startup timeout).
                eprintln!(
                    "CHAIN_ROLLBACK: loaded {loaded_count} blocks from epoch {epoch_id} but only \
                     {round} formed a valid prefix — {} block(s) discarded. Rotating to a new \
                     epoch; {epoch_id}'s remaining data is preserved untouched, not overwritten.",
                    loaded_count - recovered.len()
                );
                let new_epoch = persistence::new_epoch_id();
                persistence::set_epoch_parent(&new_epoch, &epoch_id).await;
                persistence::set_current_epoch(&new_epoch).await;
                node.chain = recovered.bound_to_window(IN_MEMORY_WINDOW);
                round = node.chain.height();
                println!(
                    "🔀 rotated to new epoch {new_epoch} (parent {epoch_id}) — resuming from \
                     height {round}"
                );
                new_epoch
            }
        }
        None => {
            // First-ever run under the epoch scheme: migrate whatever chain is
            // currently live under the old flat `blocks/{index}` collection into a
            // freshly-created epoch. The flat collection is only ever *read* here,
            // never written to again by this or any future version of this code.
            let (recovered, loaded_count) = match persistence::load_legacy_flat_chain().await {
                Some(blocks) => {
                    let loaded_count = blocks.len();
                    (Chain::recover_longest_valid_prefix(blocks), loaded_count)
                }
                None => (Chain::default(), 0),
            };
            if entropa_core::rollback_detected(loaded_count, recovered.len()) {
                eprintln!(
                    "CHAIN_ROLLBACK: legacy flat storage had {loaded_count} blocks but only \
                     {} formed a valid prefix — {} block(s) discarded during migration.",
                    recovered.len(),
                    loaded_count - recovered.len()
                );
            }
            let new_epoch = persistence::new_epoch_id();
            for b in &recovered.blocks {
                persistence::save_block(&new_epoch, b).await;
            }
            persistence::set_current_epoch(&new_epoch).await;
            node.chain = recovered.bound_to_window(IN_MEMORY_WINDOW);
            round = node.chain.height();
            println!(
                "🔀 migrated {round} block(s) from legacy flat storage into new epoch \
                 {new_epoch} — legacy storage is now frozen and will never be written to again"
            );
            new_epoch
        }
    };

    let mut state = AppState::new(node).with_active_epoch(&active_epoch);
    match std::env::var("ENTROPA_API_KEYS") {
        Ok(spec) if !spec.trim().is_empty() => {
            state = state.with_api_keys(&spec);
            println!(
                "🔐 /api/tx requires an API key — {} partner(s) configured",
                state.api_keys.len()
            );
        }
        _ => println!("🔓 /api/tx is open — set ENTROPA_API_KEYS to require partner keys"),
    }

    // Background: each round, decide → submit → produce a real Proof-of-Entropy block,
    // then persist (best-effort — a failed save just gets caught by the next one).
    let ticker = Arc::clone(&state.node);
    tokio::spawn(async move {
        let active_epoch = active_epoch;
        let mut round = round;
        let mut last_tick_log = tokio::time::Instant::now() - Duration::from_secs(TICK_LOG_SECS);
        loop {
            if last_tick_log.elapsed() >= Duration::from_secs(TICK_LOG_SECS) {
                let height = ticker.lock().unwrap().chain.height();
                println!("💓 demo node alive — height {height}, round {round}");
                last_tick_log = tokio::time::Instant::now();
            }

            let round_result = tokio::time::timeout(
                Duration::from_secs(ROUND_WATCHDOG_SECS),
                run_one_round(&ticker, &active_epoch, round),
            )
            .await;

            match round_result {
                Ok(()) => round += 1,
                Err(_) => {
                    eprintln!(
                        "⚠️ WATCHDOG: round exceeded {ROUND_WATCHDOG_SECS}s without \
                         completing — exiting so Cloud Run restarts this instance"
                    );
                    std::process::exit(1);
                }
            }
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
    });

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .expect("bind :8080");
    println!("🌌 Scryon live at http://localhost:8080  —  Entropa · Proof of Entropy · ML-DSA (FIPS-204)");
    axum::serve(listener, app(state)).await.expect("serve");
}

/// One round's work — extracted so it can be wrapped in a watchdog timeout (see
/// `ROUND_WATCHDOG_SECS`). Not independently unit-tested: real I/O (Firestore), same
/// posture as `persistence.rs`'s own async functions.
async fn run_one_round(ticker: &Arc<std::sync::Mutex<Node>>, active_epoch: &str, round: u64) {
    // Fetch the live entropy beacon (drand's public quicknet) once for this round and
    // inject it before the sync is_proposer/try_produce pair runs, so both see the
    // identical value — falls back to the deterministic stub if drand is unreachable.
    let live_beacon = entropa_core::beacon::sample_live().await;
    let produced_block = {
        let mut n = ticker.lock().unwrap();
        let tx = demo_decision(round, n.mempool.len());
        n.submit(tx);
        n.live_beacon = live_beacon;
        let block = n.try_produce(round, now());
        // Re-bound immediately after producing, inside the same lock as the append,
        // so RAM stays flat continuously rather than only at resume.
        n.chain = std::mem::take(&mut n.chain).bound_to_window(IN_MEMORY_WINDOW);
        block
    };
    if let Some(block) = produced_block {
        persistence::save_block(active_epoch, &block).await;
        for tx in &block.transactions {
            persistence::save_receipt_index(&tx.content_hash(), active_epoch, block.index).await;
        }
    }
}
