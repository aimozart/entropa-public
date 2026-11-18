//! Epoch lifecycle: creation, the current-epoch pointer, and rotation/lineage
//! (a rollback rotates to a new epoch by writing a `parent_epoch` pointer rather
//! than copying every recovered block forward — see the module doc in
//! [`super`] for why).

use entropa_core::Block;
use serde_json::{json, Value};

use super::{access_token, base_url, load_blocks_from, project};

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

pub struct EpochRecordMeta {
    pub parent_epoch: Option<String>,
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
