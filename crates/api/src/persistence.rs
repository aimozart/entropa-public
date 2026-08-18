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

/// Persist one block, with a few retries on transient failure. Still best-effort —
/// logs and returns if every attempt fails, never panics the caller — but a single
/// blip in network/auth/Firestore no longer immediately creates a permanent gap.
/// A gap here is no longer catastrophic on resume either way (see
/// `Chain::recover_longest_valid_prefix`), but preventing the gap in the first
/// place means less history gets discarded when one does slip through.
pub async fn save_block(epoch: &str, block: &Block) {
    let Some(project) = project() else { return };
    let client = reqwest::Client::new();

    const MAX_ATTEMPTS: u32 = 3;
    for attempt in 1..=MAX_ATTEMPTS {
        let Some(token) = access_token(&client).await else {
            if attempt == MAX_ATTEMPTS {
                eprintln!(
                    "persistence: firestore save failed for block {}: could not get access token \
                     after {MAX_ATTEMPTS} attempts",
                    block.index
                );
            }
            continue;
        };
        let url = format!(
            "{}/chains/{epoch}/blocks/{}",
            base_url(&project),
            block.index
        );
        match client
            .patch(&url)
            .bearer_auth(token)
            .header("Content-Type", "application/json")
            .json(&block_to_fields(block))
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => return,
            Ok(resp) => {
                if attempt == MAX_ATTEMPTS {
                    eprintln!(
                        "persistence: firestore save failed for block {}: HTTP {} after \
                         {MAX_ATTEMPTS} attempts",
                        block.index,
                        resp.status()
                    );
                }
            }
            Err(e) => {
                if attempt == MAX_ATTEMPTS {
                    eprintln!(
                        "persistence: firestore save failed for block {}: {e} after \
                         {MAX_ATTEMPTS} attempts",
                        block.index
                    );
                }
            }
        }
        if attempt < MAX_ATTEMPTS {
            tokio::time::sleep(std::time::Duration::from_millis(300 * attempt as u64)).await;
        }
    }
}

/// Load every block document under `collection_path`, ordered by index. Returns
/// `None` on any failure (no project configured, not on GCE/Cloud Run, no blocks
/// yet, bad response) — the caller falls back to starting fresh.
async fn load_blocks_from(collection_path: &str) -> Option<Vec<Block>> {
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

/// One epoch's own contribution to a chain that may span several epochs — see
/// [`reconstruct_chain`]. Fetched independently of any other epoch's data.
pub struct EpochRecord {
    pub parent_epoch: Option<String>,
    pub own_blocks: Vec<Block>,
}

/// Pure: reassemble the full, oldest-first block history for `epoch` by walking
/// `parent_epoch` pointers back through `records`, then concatenating each epoch's
/// `own_blocks` in root-to-tip order. No I/O — `records` must already contain every
/// epoch on the path from `epoch` back to its root (missing/broken links return
/// `None` rather than a partial, silently-wrong chain).
///
/// This is what lets a rollback rotate to a new epoch by writing a single pointer
/// (`parent_epoch` + where it starts) instead of copying the entire recovered
/// prefix — the same blocks stay in the old epoch, forever untouched, and get
/// stitched back in on read instead of on write.
pub fn reconstruct_chain(
    epoch: &str,
    records: &std::collections::HashMap<String, EpochRecord>,
) -> Option<Vec<Block>> {
    let mut lineage = vec![epoch.to_string()];
    let mut cursor = epoch.to_string();
    loop {
        let rec = records.get(&cursor)?;
        match &rec.parent_epoch {
            Some(parent) => {
                if lineage.contains(parent) {
                    return None; // cycle — malformed data, refuse to guess
                }
                lineage.push(parent.clone());
                cursor = parent.clone();
            }
            None => break,
        }
    }
    lineage.reverse(); // root epoch first, `epoch` itself last
    let mut blocks = Vec::new();
    for id in &lineage {
        blocks.extend(records.get(id)?.own_blocks.iter().cloned());
    }
    Some(blocks)
}

/// Load every persisted block for the given epoch, walking back through any
/// `parent_epoch` pointers (see [`reconstruct_chain`]) so a chain that has rotated
/// epochs one or more times still resolves to its full history, ordered by index.
///
/// Same fails-soft posture as the rest of this module: an epoch contributing zero
/// blocks (a freshly-rotated epoch with nothing produced yet, *or* a transient fetch
/// failure) is treated as "no new blocks from this epoch" rather than aborting the
/// whole load — consistent with every other function here choosing availability
/// over perfect precision, and safe because a genuinely missing/corrupt block still
/// gets caught downstream by `Chain::recover_longest_valid_prefix`'s own validation.
pub async fn load_chain(epoch: &str) -> Option<Vec<Block>> {
    let mut records = std::collections::HashMap::new();
    let mut cursor = epoch.to_string();
    loop {
        let own_blocks = load_blocks_from(&format!("chains/{cursor}/blocks"))
            .await
            .unwrap_or_default();
        let parent_epoch = get_epoch_info(&cursor).await.and_then(|i| i.parent_epoch);
        let go_to_parent = parent_epoch.clone();
        records.insert(
            cursor.clone(),
            EpochRecord {
                parent_epoch,
                own_blocks,
            },
        );
        match go_to_parent {
            Some(parent) if !records.contains_key(&parent) => cursor = parent,
            _ => break,
        }
    }
    reconstruct_chain(epoch, &records).filter(|b| !b.is_empty())
}

fn epoch_info_url(project: &str, epoch: &str) -> String {
    format!("{}/chains/{epoch}/meta/info", base_url(project))
}

/// Read the epoch's parent pointer, if it has one (root epochs — including every
/// epoch created before this mechanism existed — have none, and load exactly like
/// they always did).
pub async fn get_epoch_info(epoch: &str) -> Option<EpochRecordMeta> {
    let project = project()?;
    let client = reqwest::Client::new();
    let token = access_token(&client).await?;
    let resp = client
        .get(epoch_info_url(&project, epoch))
        .bearer_auth(&token)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None; // 404 — no parent, this is a root epoch
    }
    let value: Value = resp.json().await.ok()?;
    let fields = value.get("fields")?;
    Some(EpochRecordMeta {
        parent_epoch: fields
            .get("parent_epoch")
            .and_then(|v| v.get("stringValue"))
            .and_then(|v| v.as_str())
            .map(String::from),
    })
}

pub struct EpochRecordMeta {
    pub parent_epoch: Option<String>,
}

/// Pin `new_epoch`'s parent to `parent_epoch` — one small write, replacing what used
/// to be a full copy of every recovered block into the new epoch's own collection.
/// The parent epoch is never written to again; `new_epoch`'s own `blocks` collection
/// only ever holds blocks produced *after* the rotation.
pub async fn set_epoch_parent(new_epoch: &str, parent_epoch: &str) -> bool {
    let Some(project) = project() else {
        return false;
    };
    let client = reqwest::Client::new();
    let Some(token) = access_token(&client).await else {
        return false;
    };
    let body = json!({"fields": {"parent_epoch": {"stringValue": parent_epoch}}});
    client
        .patch(epoch_info_url(&project, new_epoch))
        .bearer_auth(token)
        .header("Content-Type", "application/json")
        .json(&body)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

/// Read-only: load whatever is sitting in the old, pre-epoch flat `blocks/{index}`
/// collection. Only ever called once, at migration time, to seed the first epoch
/// with whatever chain was live under the old scheme — never written to again
/// after that, so this function existing is not itself a re-introduction of the
/// original hazard.
pub async fn load_legacy_flat_chain() -> Option<Vec<Block>> {
    load_blocks_from("blocks").await
}

fn epoch_doc_url(project: &str) -> String {
    format!("{}/meta/current_epoch", base_url(project))
}

/// Read the currently-active epoch ID, if one has ever been set.
pub async fn get_current_epoch() -> Option<String> {
    let project = project()?;
    let client = reqwest::Client::new();
    let token = access_token(&client).await?;
    let resp = client
        .get(epoch_doc_url(&project))
        .bearer_auth(&token)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None; // includes 404 — no epoch has been created yet
    }
    let value: Value = resp.json().await.ok()?;
    value
        .get("fields")?
        .get("epoch_id")?
        .get("stringValue")?
        .as_str()
        .map(String::from)
}

/// Pin `epoch_id` as the currently-active epoch. Called once when a new epoch is
/// created (first-ever run, or a rollback forced a rotation) — never overwritten
/// with an older epoch's ID, since the pointer only ever moves forward.
pub async fn set_current_epoch(epoch_id: &str) -> bool {
    let Some(project) = project() else {
        return false;
    };
    let client = reqwest::Client::new();
    let Some(token) = access_token(&client).await else {
        return false;
    };
    let body = json!({"fields": {"epoch_id": {"stringValue": epoch_id}}});
    client
        .patch(epoch_doc_url(&project))
        .bearer_auth(token)
        .header("Content-Type", "application/json")
        .json(&body)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

/// Generate a new, essentially-unique epoch ID. Timestamp-based so epochs sort
/// chronologically and are human-readable in the Firestore console.
pub fn new_epoch_id() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("epoch-{now}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn block(index: u64) -> Block {
        Block {
            index,
            timestamp: index,
            prev_hash: format!("prev-{index}"),
            beacon: "beacon".to_string(),
            transactions: vec![],
            proposer_id: "PROBE-TEST".to_string(),
            proposer_pubkey: "pubkey".to_string(),
            hash: format!("hash-{index}"),
            signature: "sig".to_string(),
        }
    }

    fn indices(blocks: &[Block]) -> Vec<u64> {
        blocks.iter().map(|b| b.index).collect()
    }

    #[test]
    fn root_epoch_with_no_parent_returns_its_own_blocks() {
        let mut records = HashMap::new();
        records.insert(
            "epoch-a".to_string(),
            EpochRecord {
                parent_epoch: None,
                own_blocks: vec![block(0), block(1), block(2)],
            },
        );
        let result = reconstruct_chain("epoch-a", &records).unwrap();
        assert_eq!(indices(&result), vec![0, 1, 2]);
    }

    #[test]
    fn single_rotation_stitches_parent_before_child() {
        let mut records = HashMap::new();
        records.insert(
            "epoch-old".to_string(),
            EpochRecord {
                parent_epoch: None,
                own_blocks: vec![block(0), block(1), block(2)],
            },
        );
        records.insert(
            "epoch-new".to_string(),
            EpochRecord {
                parent_epoch: Some("epoch-old".to_string()),
                own_blocks: vec![block(3), block(4)],
            },
        );
        let result = reconstruct_chain("epoch-new", &records).unwrap();
        assert_eq!(indices(&result), vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn multiple_rotations_deep_still_resolve_oldest_first() {
        let mut records = HashMap::new();
        records.insert(
            "epoch-1".to_string(),
            EpochRecord {
                parent_epoch: None,
                own_blocks: vec![block(0), block(1)],
            },
        );
        records.insert(
            "epoch-2".to_string(),
            EpochRecord {
                parent_epoch: Some("epoch-1".to_string()),
                own_blocks: vec![block(2), block(3)],
            },
        );
        records.insert(
            "epoch-3".to_string(),
            EpochRecord {
                parent_epoch: Some("epoch-2".to_string()),
                own_blocks: vec![block(4)],
            },
        );
        let result = reconstruct_chain("epoch-3", &records).unwrap();
        assert_eq!(indices(&result), vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn freshly_rotated_epoch_with_no_new_blocks_yet_still_returns_full_parent_history() {
        // Exactly the state right after a rollback: the pointer is written, but no
        // new block has been produced under the new epoch yet.
        let mut records = HashMap::new();
        records.insert(
            "epoch-old".to_string(),
            EpochRecord {
                parent_epoch: None,
                own_blocks: vec![block(0), block(1)],
            },
        );
        records.insert(
            "epoch-new".to_string(),
            EpochRecord {
                parent_epoch: Some("epoch-old".to_string()),
                own_blocks: vec![],
            },
        );
        let result = reconstruct_chain("epoch-new", &records).unwrap();
        assert_eq!(indices(&result), vec![0, 1]);
    }

    #[test]
    fn missing_link_in_the_lineage_returns_none_rather_than_a_partial_chain() {
        let mut records = HashMap::new();
        // "epoch-new" points at a parent that was never fetched/doesn't exist.
        records.insert(
            "epoch-new".to_string(),
            EpochRecord {
                parent_epoch: Some("epoch-ghost".to_string()),
                own_blocks: vec![block(5)],
            },
        );
        assert!(reconstruct_chain("epoch-new", &records).is_none());
    }

    #[test]
    fn unknown_epoch_returns_none() {
        let records = HashMap::new();
        assert!(reconstruct_chain("nothing-here", &records).is_none());
    }

    #[test]
    fn cyclic_parent_pointers_refuse_to_guess_rather_than_looping_forever() {
        let mut records = HashMap::new();
        records.insert(
            "epoch-a".to_string(),
            EpochRecord {
                parent_epoch: Some("epoch-b".to_string()),
                own_blocks: vec![block(0)],
            },
        );
        records.insert(
            "epoch-b".to_string(),
            EpochRecord {
                parent_epoch: Some("epoch-a".to_string()),
                own_blocks: vec![block(1)],
            },
        );
        assert!(reconstruct_chain("epoch-a", &records).is_none());
    }
}
