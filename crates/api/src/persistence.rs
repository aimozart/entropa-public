//! GCS-backed chain persistence — so the network survives a redeploy.
//!
//! Deliberately minimal: no cloud SDK, just two GCS JSON API calls authenticated via
//! the Cloud Run metadata server's Application Default Credentials. Matches the
//! project's own principle — exactly as much infrastructure as the job requires, no
//! more. Entirely optional: if `ENTROPA_GCS_BUCKET` isn't set (e.g. local `cargo run`),
//! or the metadata server isn't reachable (anywhere off Cloud Run/GCE), every call
//! fails soft and the network just runs in-memory, as before.

use entropa_core::Block;

const METADATA_TOKEN_URL: &str =
    "http://metadata.google.internal/computeMetadata/v1/instance/service-accounts/default/token";
const OBJECT_NAME: &str = "chain.json";

fn bucket() -> Option<String> {
    std::env::var("ENTROPA_GCS_BUCKET").ok()
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
    resp.json::<TokenResponse>().await.ok().map(|t| t.access_token)
}

/// Load the persisted chain, if a bucket is configured and an object exists. Returns
/// `None` on any failure (no bucket configured, not on GCE/Cloud Run, object missing,
/// bad JSON) — the caller falls back to starting fresh.
pub async fn load_chain() -> Option<Vec<Block>> {
    let bucket = bucket()?;
    let client = reqwest::Client::new();
    let token = access_token(&client).await?;
    let url = format!(
        "https://storage.googleapis.com/storage/v1/b/{bucket}/o/{OBJECT_NAME}?alt=media"
    );
    let resp = client
        .get(&url)
        .bearer_auth(token)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None; // e.g. 404 — no persisted state yet, that's fine
    }
    resp.json::<Vec<Block>>().await.ok()
}

/// Persist the current chain. Best-effort — logs and returns on failure, never panics
/// the caller; losing one save just means the next successful save catches up.
pub async fn save_chain(blocks: &[Block]) {
    let Some(bucket) = bucket() else { return };
    let client = reqwest::Client::new();
    let Some(token) = access_token(&client).await else {
        return;
    };
    let url = format!(
        "https://storage.googleapis.com/upload/storage/v1/b/{bucket}/o?uploadType=media&name={OBJECT_NAME}"
    );
    let body = match serde_json::to_vec(blocks) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("persistence: failed to serialize chain: {e}");
            return;
        }
    };
    if let Err(e) = client
        .post(&url)
        .bearer_auth(token)
        .header("Content-Type", "application/json")
        .body(body)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
    {
        eprintln!("persistence: save failed: {e}");
    }
}
