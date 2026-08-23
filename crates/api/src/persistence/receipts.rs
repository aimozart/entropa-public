//! The receipt index (`receipts/{content_hash} -> {epoch, block_index}`) and the
//! evicted-block fallback lookups built on top of it — what lets `GET
//! /api/receipt/{id}` and `/api/chain` pagination keep working for history that
//! `Chain::bound_to_window` has evicted from RAM.

use entropa_core::Block;
use serde_json::{json, Value};

use super::epochs::get_epoch_info;
use super::{access_token, base_url, fields_to_block, project};

fn receipt_index_url(project: &str, content_hash: &str) -> String {
    format!("{}/receipts/{content_hash}", base_url(project))
}

/// Pure: encode a receipt-index location as Firestore document fields. Separated
/// from the I/O so the encoding itself is unit-testable without a live Firestore.
fn receipt_index_fields(epoch: &str, block_index: u64) -> Value {
    json!({
        "fields": {
            "epoch": {"stringValue": epoch},
            "block_index": {"integerValue": block_index.to_string()},
        }
    })
}

/// Pure: decode a receipt-index location back out of a Firestore document response.
/// `None` on any malformed/missing field, same fails-soft posture as the rest of
/// this module.
fn parse_receipt_index_fields(value: &Value) -> Option<(String, u64)> {
    let fields = value.get("fields")?;
    let epoch = fields
        .get("epoch")?
        .get("stringValue")?
        .as_str()?
        .to_string();
    let block_index = fields
        .get("block_index")?
        .get("integerValue")?
        .as_str()?
        .parse()
        .ok()?;
    Some((epoch, block_index))
}

/// Pin a transaction's location (`epoch`, `block_index`) so its receipt stays a
/// single indexed lookup forever, regardless of how far the in-memory chain window
/// (`Chain::bound_to_window`) has moved past it. Written once, alongside the block
/// that contains the transaction — never overwritten after.
pub async fn save_receipt_index(content_hash: &str, epoch: &str, block_index: u64) -> bool {
    let Some(project) = project() else {
        return false;
    };
    let client = reqwest::Client::new();
    let Some(token) = access_token(&client).await else {
        return false;
    };
    client
        .patch(receipt_index_url(&project, content_hash))
        .bearer_auth(token)
        .header("Content-Type", "application/json")
        .json(&receipt_index_fields(epoch, block_index))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

/// Read back where a transaction's block lives, if it was ever indexed. `None` for
/// an unknown hash (including transactions submitted before this index existed —
/// callers fall back to whatever they did before this fix).
pub async fn get_receipt_location(content_hash: &str) -> Option<(String, u64)> {
    let project = project()?;
    let client = reqwest::Client::new();
    let token = access_token(&client).await?;
    let resp = client
        .get(receipt_index_url(&project, content_hash))
        .bearer_auth(&token)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let value: Value = resp.json().await.ok()?;
    parse_receipt_index_fields(&value)
}

/// Load exactly one block by its epoch and index — used by the receipt fallback
/// path to fetch a single historical block without loading everything around it.
pub async fn load_block(epoch: &str, block_index: u64) -> Option<Block> {
    let project = project()?;
    let client = reqwest::Client::new();
    let token = access_token(&client).await?;
    let url = format!("{}/chains/{epoch}/blocks/{block_index}", base_url(&project));
    let resp = client
        .get(&url)
        .bearer_auth(&token)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let doc: Value = resp.json().await.ok()?;
    fields_to_block(&doc)
}

/// Load block `block_index` starting from `epoch`, walking up through
/// `parent_epoch` pointers if it isn't found there — a rotation only ever records
/// a pointer rather than copying every ancestor's blocks forward (see this
/// module's doc), so a global index produced before the most recent rotation(s)
/// lives under an ancestor epoch's own `blocks` collection, not the current one.
/// Bounded by the lineage depth (the number of rotations ever recorded), not the
/// chain's height — cheap for the rare deep-history read this exists for
/// (`/api/chain` pagination below the in-memory window's floor).
pub async fn load_block_in_lineage(epoch: &str, block_index: u64) -> Option<Block> {
    let mut cursor = epoch.to_string();
    let mut visited = std::collections::HashSet::new();
    loop {
        if let Some(block) = load_block(&cursor, block_index).await {
            return Some(block);
        }
        if !visited.insert(cursor.clone()) {
            return None; // cycle — malformed data, refuse to loop forever
        }
        match get_epoch_info(&cursor).await.and_then(|i| i.parent_epoch) {
            Some(parent) => cursor = parent,
            None => return None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_index_fields_round_trip() {
        let encoded = receipt_index_fields("epoch-abc", 42);
        let decoded = parse_receipt_index_fields(&encoded);
        assert_eq!(decoded, Some(("epoch-abc".to_string(), 42)));
    }

    #[test]
    fn receipt_index_fields_round_trip_preserves_large_index() {
        // u64 block indices must survive the string round-trip Firestore's
        // integerValue encoding requires - a naive implementation could silently
        // truncate at a smaller integer width.
        let encoded = receipt_index_fields("epoch-xyz", u64::MAX);
        let decoded = parse_receipt_index_fields(&encoded);
        assert_eq!(decoded, Some(("epoch-xyz".to_string(), u64::MAX)));
    }

    #[test]
    fn parse_receipt_index_fields_rejects_missing_epoch() {
        let malformed = json!({"fields": {"block_index": {"integerValue": "5"}}});
        assert_eq!(parse_receipt_index_fields(&malformed), None);
    }

    #[test]
    fn parse_receipt_index_fields_rejects_missing_block_index() {
        let malformed = json!({"fields": {"epoch": {"stringValue": "epoch-abc"}}});
        assert_eq!(parse_receipt_index_fields(&malformed), None);
    }

    #[test]
    fn parse_receipt_index_fields_rejects_non_numeric_block_index() {
        let malformed = json!({
            "fields": {
                "epoch": {"stringValue": "epoch-abc"},
                "block_index": {"integerValue": "not-a-number"},
            }
        });
        assert_eq!(parse_receipt_index_fields(&malformed), None);
    }

    #[test]
    fn parse_receipt_index_fields_rejects_missing_fields_wrapper() {
        assert_eq!(parse_receipt_index_fields(&json!({})), None);
    }
}
