//! Shared application state (`AppState`) and its per-partner rate-limit bookkeeping.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use entropa_node::Node;

/// A token bucket for one partner's per-key rate limit: refills continuously at
/// `PER_KEY_RATE` tokens/second up to `PER_KEY_BURST` capacity; each request costs one
/// token. Simpler than a sliding-window log (O(1) per check, not O(requests-in-window)),
/// and doesn't need a background sweep task the way a fixed-window counter would.
pub(crate) struct RateBucket {
    tokens: f64,
    last_refill: std::time::Instant,
}

pub(crate) const PER_KEY_RATE: f64 = 20.0;
pub(crate) const PER_KEY_BURST: f64 = 20.0;

/// `true` if the request is allowed (and consumes a token); `false` if this partner is
/// over their own rate limit right now. Independent per partner — one partner maxing out
/// their budget has zero effect on any other partner's.
pub(crate) fn check_per_key_rate_limit(
    buckets: &Mutex<HashMap<String, RateBucket>>,
    key: &str,
) -> bool {
    let mut map = buckets.lock().unwrap();
    let now = std::time::Instant::now();
    let bucket = map.entry(key.to_string()).or_insert_with(|| RateBucket {
        tokens: PER_KEY_BURST,
        last_refill: now,
    });
    let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
    bucket.tokens = (bucket.tokens + elapsed * PER_KEY_RATE).min(PER_KEY_BURST);
    bucket.last_refill = now;
    if bucket.tokens >= 1.0 {
        bucket.tokens -= 1.0;
        true
    } else {
        false
    }
}

/// Shared application state: the node whose chain we serve.
#[derive(Clone)]
pub struct AppState {
    pub node: Arc<Mutex<Node>>,
    /// Bearer key → owning partner name. Empty means `/api/tx` is open (dev/demo mode).
    pub api_keys: Arc<HashMap<String, String>>,
    /// Per-partner rate-limit state, keyed by partner name (not the raw API key, so
    /// rotating a partner's key doesn't reset their budget mid-window). Sits on top of
    /// the global 20 req/s cap on the whole route — that one protects the service from
    /// being hammered at all; this one protects partners from crowding each other out
    /// underneath that ceiling.
    pub(crate) rate_buckets: Arc<Mutex<HashMap<String, RateBucket>>>,
}

impl AppState {
    pub fn new(node: Node) -> Self {
        Self {
            node: Arc::new(Mutex::new(node)),
            api_keys: Arc::new(HashMap::new()),
            rate_buckets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Parse `ENTROPA_API_KEYS`-style config: `"name1:key1,name2:key2"`.
    pub fn with_api_keys(mut self, spec: &str) -> Self {
        let keys = spec
            .split(',')
            .filter_map(|pair| {
                let (name, key) = pair.split_once(':')?;
                let (name, key) = (name.trim(), key.trim());
                (!name.is_empty() && !key.is_empty()).then(|| (key.to_string(), name.to_string()))
            })
            .collect();
        self.api_keys = Arc::new(keys);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression test for per-key rate limiting: one partner exhausting their own budget
    /// must have zero effect on any other partner's budget -- the whole point of moving
    /// off the single shared global bucket.
    #[test]
    fn per_key_rate_limit_is_independent_per_partner() {
        let buckets: Mutex<HashMap<String, RateBucket>> = Mutex::new(HashMap::new());

        for _ in 0..(PER_KEY_BURST as usize) {
            assert!(check_per_key_rate_limit(&buckets, "acme"));
        }
        assert!(
            !check_per_key_rate_limit(&buckets, "acme"),
            "acme should be out of budget after exhausting its burst capacity"
        );

        assert!(
            check_per_key_rate_limit(&buckets, "other-partner"),
            "a different partner must be unaffected by acme's exhausted budget"
        );
    }
}
