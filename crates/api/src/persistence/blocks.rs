//! Raw block save/load against Firestore — the two entry points beyond the shared
//! [`super::load_blocks_from`] helper.

use entropa_core::Block;
use serde_json::Value;

use super::{access_token, base_url, block_to_fields, fields_to_block, load_blocks_from, project};

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

/// Read-only: load whatever is sitting in the old, pre-epoch flat `blocks/{index}`
/// collection. Only ever called once, at migration time, to seed the first epoch
/// with whatever chain was live under the old scheme — never written to again
/// after that, so this function existing is not itself a re-introduction of the
/// original hazard.
pub async fn load_legacy_flat_chain() -> Option<Vec<Block>> {
    load_blocks_from("blocks").await
}

/// Pure: split a Firestore page already ordered by index **descending** into the
/// most recent `window` blocks (re-ordered ascending, the shape callers actually
/// want) plus the floor block immediately preceding the window, if the full set
/// is larger than `window`. `None` floor means the input already covered the
/// whole chain (nothing was cut).
fn split_recent_window(mut descending: Vec<Block>, window: usize) -> (Vec<Block>, Option<Block>) {
    if descending.len() <= window {
        descending.reverse();
        return (descending, None);
    }
    let rest = descending.split_off(window);
    let floor = rest.into_iter().next();
    descending.reverse();
    (descending, floor)
}

/// Load only the most recent `window` blocks under `epoch`, plus the block
/// immediately preceding the window (as `(recent_blocks_ascending,
/// floor_block)`) — bounds resume-time Firestore reads to O(window) instead of
/// O(full chain height). This is the fix for a real production incident: at
/// height ~176,712, a full-history resume (`load_chain`) exceeded Cloud Run's
/// startup probe timeout. `None` on any failure (no project, off Cloud Run/GCE,
/// no blocks yet) — same fails-soft posture as the rest of this module.
pub async fn load_recent_blocks(epoch: &str, window: usize) -> Option<(Vec<Block>, Option<Block>)> {
    let project = project()?;
    let client = reqwest::Client::new();
    let token = access_token(&client).await?;

    let mut descending = Vec::new();
    let mut page_token: Option<String> = None;
    loop {
        let mut url = format!(
            "{}/chains/{epoch}/blocks?pageSize=300&orderBy=index%20desc",
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
                    descending.push(b);
                }
            }
        }
        // Stop as soon as we have enough for the window (+1 for the floor) —
        // no need to keep paginating through older history at all.
        if descending.len() > window {
            break;
        }
        page_token = value
            .get("nextPageToken")
            .and_then(|t| t.as_str())
            .map(String::from);
        if page_token.is_none() {
            break;
        }
    }

    if descending.is_empty() {
        return None;
    }
    Some(split_recent_window(descending, window))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block_at(index: u64) -> Block {
        Block {
            index,
            timestamp: 1_000 + index,
            prev_hash: format!("hash-{}", index.wrapping_sub(1)),
            beacon: "COSMIC-test".to_string(),
            transactions: Vec::new(),
            proposer_id: "PROBE-TEST".to_string(),
            proposer_pubkey: "deadbeef".to_string(),
            hash: format!("hash-{index}"),
            signature: "irrelevant-for-this-test".to_string(),
        }
    }

    // --- Phase 4g: split_recent_window ---
    //
    // Pure split of a Firestore descending-by-index page into the ascending
    // window a caller actually wants plus the floor block just before it - the
    // fix for a real production incident (resume at height ~176,712 exceeded
    // Cloud Run's startup probe timeout reading/validating the entire history).

    #[test]
    fn fewer_blocks_than_window_returns_everything_with_no_floor() {
        let descending = vec![block_at(2), block_at(1), block_at(0)];
        let (window, floor) = split_recent_window(descending, 5);
        let indices: Vec<u64> = window.iter().map(|b| b.index).collect();
        assert_eq!(indices, vec![0, 1, 2], "must be reordered ascending");
        assert!(floor.is_none());
    }

    #[test]
    fn exactly_window_sized_input_returns_everything_with_no_floor() {
        let descending = vec![block_at(2), block_at(1), block_at(0)];
        let (window, floor) = split_recent_window(descending, 3);
        assert_eq!(window.len(), 3);
        assert!(floor.is_none());
    }

    #[test]
    fn more_blocks_than_window_returns_only_the_most_recent_plus_a_floor() {
        // Indices 0..=9 descending; window=3 should keep 7,8,9 and float 6.
        let descending: Vec<Block> = (0..10).rev().map(block_at).collect();
        let (window, floor) = split_recent_window(descending, 3);
        let indices: Vec<u64> = window.iter().map(|b| b.index).collect();
        assert_eq!(indices, vec![7, 8, 9]);
        assert_eq!(floor.map(|b| b.index), Some(6));
    }

    #[test]
    fn empty_input_returns_empty_window_and_no_floor() {
        let (window, floor) = split_recent_window(vec![], 5);
        assert!(window.is_empty());
        assert!(floor.is_none());
    }

    #[test]
    fn zero_window_still_returns_a_correct_floor() {
        let descending: Vec<Block> = (0..3).rev().map(block_at).collect();
        let (window, floor) = split_recent_window(descending, 0);
        assert!(window.is_empty());
        assert_eq!(floor.map(|b| b.index), Some(2));
    }
}
