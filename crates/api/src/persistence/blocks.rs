//! Raw block save/load against Firestore — the two entry points beyond the shared
//! [`super::load_blocks_from`] helper.

use entropa_core::Block;

use super::{access_token, base_url, block_to_fields, load_blocks_from, project};

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
