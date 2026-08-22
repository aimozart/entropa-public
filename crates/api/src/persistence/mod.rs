//! Firestore-backed chain persistence — so the network survives a redeploy.
//!
//! Blocks live under an **epoch namespace**: `chains/{epoch_id}/blocks/{index}`,
//! never a flat `blocks/{index}`. This is the load-bearing safety property of this
//! module — see the 2026-08-13 incident where a flat, index-only key scheme let a
//! restarted chain silently overwrite a prior chain's history at the same indices,
//! destroying ~27,000 blocks' worth of audit trail. An epoch ID is generated once
//! and pinned in `meta/current_epoch`; every block this process ever writes goes
//! under that one epoch's namespace. If a rollback is ever detected on resume
//! (`Chain::recover_longest_valid_prefix` had to discard anything), the caller
//! rotates to a **brand new epoch** rather than continuing to write into the same
//! paths — so a future incident costs at most the *current* epoch's un-recovered
//! tail, and can structurally never touch a previous epoch's data. Every past
//! epoch remains permanently stored and queryable; nothing is ever silently
//! destroyed again.
//!
//! **Rotation is a pointer write, not a copy** (`set_epoch_parent`/`reconstruct_chain`,
//! added 2026-08-18 after a live resilience test proved the original copy-every-
//! recovered-block approach exceeds Cloud Run's startup timeout once the chain gets
//! tall). The new epoch records `parent_epoch` and starts its own `blocks`
//! collection empty; `load_chain` walks parent pointers back to the root and
//! stitches each epoch's own blocks together, oldest first. The old epoch is never
//! written to again either way — only *how* history before the rotation point gets
//! read back changed, not the never-touch-the-past guarantee itself.
//!
//! Each block is its own document, written once and (within an epoch) never
//! rewritten except by its own producer. A save is one small write, not a
//! re-upload of the entire chain history (the earlier GCS-single-blob approach did
//! the latter, and would only have gotten slower and more expensive as the chain
//! grew). Resume reads every document back, ordered by index.
//!
//! Deliberately minimal: no cloud SDK, just the Firestore REST API authenticated via
//! the Cloud Run metadata server's Application Default Credentials — same pattern as
//! the rest of this project's GCP calls. Entirely optional: if `GCP_PROJECT` isn't set
//! (e.g. local `cargo run`), or the metadata server isn't reachable (anywhere off
//! Cloud Run/GCE), every call fails soft and the network just runs in-memory.
//!
//! Split into submodules by responsibility (`CLAUDE.md` L20 — this file was 784 lines
//! before the 2026-08-22 split): [`blocks`] (raw block save/load), [`epochs`] (epoch
//! lifecycle, lineage/rotation), [`receipts`] (the receipt index + evicted-block
//! fallback lookups). Everything below this doc comment is genuinely shared plumbing
//! (auth, wire-format encode/decode) used by all three, which is why it stays here
//! rather than picking one submodule to own it.

mod blocks;
mod epochs;
mod receipts;

pub use blocks::{load_legacy_flat_chain, save_block};
pub use epochs::{
    get_current_epoch, get_epoch_info, load_chain, new_epoch_id, reconstruct_chain,
    set_current_epoch, set_epoch_parent, EpochRecord, EpochRecordMeta,
};
pub use receipts::{get_receipt_location, load_block, load_block_in_lineage, save_receipt_index};

use entropa_core::{Block, Transaction};
use serde_json::{json, Value};

const METADATA_TOKEN_URL: &str =
    "http://metadata.google.internal/computeMetadata/v1/instance/service-accounts/default/token";

pub(crate) fn project() -> Option<String> {
    std::env::var("GCP_PROJECT").ok()
}

pub(crate) fn base_url(project: &str) -> String {
    format!("https://firestore.googleapis.com/v1/projects/{project}/databases/(default)/documents")
}

pub(crate) async fn access_token(client: &reqwest::Client) -> Option<String> {
    #[derive(serde::Deserialize)]
    struct TokenResponse {
        access_token: String,
    }
    let resp = client
        .get(METADATA_TOKEN_URL)
        .header("Metadata-Flavor", "Google")
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    resp.json::<TokenResponse>()
        .await
        .ok()
        .map(|t| t.access_token)
}

pub(crate) fn block_to_fields(block: &Block) -> Value {
    let txs: Vec<Value> = block
        .transactions
        .iter()
        .map(|tx| {
            json!({
                "mapValue": { "fields": {
                    "from": {"stringValue": tx.from},
                    "kind": {"stringValue": tx.kind},
                    "payload": {"stringValue": tx.payload},
                }}
            })
        })
        .collect();
    json!({
        "fields": {
            "index": {"integerValue": block.index.to_string()},
            "timestamp": {"integerValue": block.timestamp.to_string()},
            "prev_hash": {"stringValue": block.prev_hash},
            "beacon": {"stringValue": block.beacon},
            "proposer_id": {"stringValue": block.proposer_id},
            "proposer_pubkey": {"stringValue": block.proposer_pubkey},
            "hash": {"stringValue": block.hash},
            "signature": {"stringValue": block.signature},
            "transactions": {"arrayValue": {"values": txs}},
        }
    })
}

pub(crate) fn fields_to_block(doc: &Value) -> Option<Block> {
    let f = doc.get("fields")?;
    let get_str =
        |k: &str| -> Option<String> { f.get(k)?.get("stringValue")?.as_str().map(String::from) };
    let get_int =
        |k: &str| -> Option<u64> { f.get(k)?.get("integerValue")?.as_str()?.parse().ok() };

    let transactions = f
        .get("transactions")
        .and_then(|t| t.get("arrayValue"))
        .and_then(|a| a.get("values"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    let tf = v.get("mapValue")?.get("fields")?;
                    Some(Transaction::new(
                        tf.get("from")?.get("stringValue")?.as_str()?.to_string(),
                        tf.get("kind")?.get("stringValue")?.as_str()?.to_string(),
                        tf.get("payload")?.get("stringValue")?.as_str()?.to_string(),
                    ))
                })
                .collect()
        })
        .unwrap_or_default();

    Some(Block {
        index: get_int("index")?,
        timestamp: get_int("timestamp")?,
        prev_hash: get_str("prev_hash")?,
        beacon: get_str("beacon")?,
        transactions,
        proposer_id: get_str("proposer_id")?,
        proposer_pubkey: get_str("proposer_pubkey")?,
        hash: get_str("hash")?,
        signature: get_str("signature")?,
    })
}

/// Load every block document under `collection_path`, ordered by index. Returns
/// `None` on any failure (no project configured, not on GCE/Cloud Run, no blocks
/// yet, bad response) — the caller falls back to starting fresh.
pub(crate) async fn load_blocks_from(collection_path: &str) -> Option<Vec<Block>> {
    let project = project()?;
    let client = reqwest::Client::new();
    let token = access_token(&client).await?;

    let mut blocks = Vec::new();
    let mut page_token: Option<String> = None;
    loop {
        let mut url = format!(
            "{}/{collection_path}?pageSize=300&orderBy=index",
            base_url(&project)
        );
        if let Some(pt) = &page_token {
            url.push_str(&format!("&pageToken={pt}"));
        }
        let resp = client
            .get(&url)
            .bearer_auth(&token)
            .timeout(std::time::Duration::from_secs(15))
            .send()
            .await
            .ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let value: Value = resp.json().await.ok()?;
        if let Some(docs) = value.get("documents").and_then(|d| d.as_array()) {
            for doc in docs {
                if let Some(b) = fields_to_block(doc) {
                    blocks.push(b);
                }
            }
        }
        page_token = value
            .get("nextPageToken")
            .and_then(|t| t.as_str())
            .map(String::from);
        if page_token.is_none() {
            break;
        }
    }

    if blocks.is_empty() {
        return None;
    }
    Some(blocks)
}
