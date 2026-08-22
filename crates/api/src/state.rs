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
    /// Partner name → Stripe customer ID, for metered billing (`billing::record_attestation`).
    /// Empty/missing entries just mean that partner isn't billed — never blocks the request.
    pub stripe_customers: Arc<HashMap<String, String>>,
    /// Bearer key → owning partner name. Empty means `/api/tx` is open (dev/demo mode).
    pub api_keys: Arc<HashMap<String, String>>,
    /// Per-partner rate-limit state, keyed by partner name (not the raw API key, so
    /// rotating a partner's key doesn't reset their budget mid-window). Sits on top of
    /// the global 20 req/s cap on the whole route — that one protects the service from
    /// being hammered at all; this one protects partners from crowding each other out
    /// underneath that ceiling.
    pub(crate) rate_buckets: Arc<Mutex<HashMap<String, RateBucket>>>,
    /// The epoch this process is currently writing new blocks under (see
    /// `persistence.rs`'s module doc) — `None` in tests/local runs where no
    /// persistence is wired up. Lets read routes (`/api/chain`, receipts, the
    /// partner dashboard) start a Firestore lineage walk from the right place when
    /// a requested block has aged out of the in-memory window.
    pub active_epoch: Option<Arc<str>>,
}

impl AppState {
    pub fn new(node: Node) -> Self {
        Self {
            node: Arc::new(Mutex::new(node)),
            stripe_customers: Arc::new(HashMap::new()),
            api_keys: Arc::new(HashMap::new()),
            rate_buckets: Arc::new(Mutex::new(HashMap::new())),
            active_epoch: None,
        }
    }

    /// Record which epoch this process is currently producing blocks under, so
    /// read routes can fall back to Firestore for history below the in-memory
    /// window's floor.
    pub fn with_active_epoch(mut self, epoch: &str) -> Self {
        self.active_epoch = Some(Arc::from(epoch));
        self
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

    /// Parse `ENTROPA_STRIPE_CUSTOMERS`-style config: `"name1:cus_xxx,name2:cus_yyy"`.
    /// A partner with no entry here just doesn't get billed — never blocks the request.
    pub fn with_stripe_customers(mut self, spec: &str) -> Self {
        let customers = spec
            .split(',')
            .filter_map(|pair| {
                let (name, cust_id) = pair.split_once(':')?;
                let (name, cust_id) = (name.trim(), cust_id.trim());
                (!name.is_empty() && !cust_id.is_empty())
                    .then(|| (name.to_string(), cust_id.to_string()))
            })
            .collect();
        self.stripe_customers = Arc::new(customers);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::node_with_one_block;

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

    #[test]
    fn stripe_customers_parses_correctly() {
        let state = AppState::new(node_with_one_block())
            .with_stripe_customers("acme:cus_123, other:cus_456");
        assert_eq!(
            state.stripe_customers.get("acme").map(String::as_str),
            Some("cus_123")
        );
        assert_eq!(
            state.stripe_customers.get("other").map(String::as_str),
            Some("cus_456")
        );
    }
}
