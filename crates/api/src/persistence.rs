//! Firestore-backed chain persistence — so the network survives a redeploy.
//!
//! Each block is its own document (`blocks/{index}`), written once and never
//! rewritten. A save is one small write, not a re-upload of the entire chain history
//! (the earlier GCS-single-blob approach did the latter, and would only have gotten
//! slower and more expensive as the chain grew). Resume reads every document back,
//! ordered by index.
//!
//! Deliberately minimal: no cloud SDK, just the Firestore REST API authenticated via
//! the Cloud Run metadata server's Application Default Credentials — same pattern as
//! the rest of this project's GCP calls. Entirely optional: if `GCP_PROJECT` isn't set
//! (e.g. local `cargo run`), or the metadata server isn't reachable (anywhere off
//! Cloud Run/GCE), every call fails soft and the network just runs in-memory.

use entropa_core::{Block, Transaction};
use serde_json::{json, Value};

const METADATA_TOKEN_URL: &str =
    "http://metadata.google.internal/computeMetadata/v1/instance/service-accounts/default/token";

fn project() -> Option<String> {
    std::env::var("GCP_PROJECT").ok()
}

fn base_url(project: &str) -> String {
    format!("https://firestore.googleapis.com/v1/projects/{project}/databases/(default)/documents")
}

async fn access_token(client: &reqwest::Client) -> Option<String> {
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

fn block_to_fields(block: &Block) -> Value {
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

fn fields_to_block(doc: &Value) -> Option<Block> {
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

/// Persist one block. Best-effort — logs and returns on failure, never panics the
/// caller; losing one save just means the next block's save catches up (this block
/// would be missing on resume, but the chain's own hash-link verification would
/// reject a gap anyway, so a missed save fails safe).
pub async fn save_block(block: &Block) {
    let Some(project) = project() else { return };
    let client = reqwest::Client::new();
    let Some(token) = access_token(&client).await else {
        return;
    };
    let url = format!("{}/blocks/{}", base_url(&project), block.index);
    if let Err(e) = client
        .patch(&url)
        .bearer_auth(token)
        .header("Content-Type", "application/json")
        .json(&block_to_fields(block))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
    {
        eprintln!("persistence: firestore save failed: {e}");
    }
}

/// Load every persisted block, ordered by index. Returns `None` on any failure (no
/// project configured, not on GCE/Cloud Run, no blocks yet, bad response) — the
/// caller falls back to starting fresh.
pub async fn load_chain() -> Option<Vec<Block>> {
    let project = project()?;
    let client = reqwest::Client::new();
    let token = access_token(&client).await?;

    let mut blocks = Vec::new();
    let mut page_token: Option<String> = None;
    loop {
        let mut url = format!("{}/blocks?pageSize=300&orderBy=index", base_url(&project));
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
