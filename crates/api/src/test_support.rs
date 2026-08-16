//! Shared test-only helpers, used across `lib.rs` and `routes/*`'s own test modules.

use entropa_core::{Probe, Transaction};
use entropa_node::{Node, Validator};

pub(crate) fn node_with_one_block() -> Node {
    let probe = Probe::spawn();
    let validators = vec![Validator::new(probe.id(), probe.pubkey_hex())];
    let mut node = Node::new(probe, validators);
    node.submit(Transaction::new("boot", "genesis", "big bang"));
    node.try_produce(0, 1_000);
    node
}

/// A node with one block containing a single transaction from `partner` — for
/// exercising `/dashboard/{partner}` without needing the real submit/produce cycle.
pub(crate) fn node_with_partner_tx(partner: &str, kind: &str, payload: &str) -> Node {
    let probe = Probe::spawn();
    let validators = vec![Validator::new(probe.id(), probe.pubkey_hex())];
    let mut node = Node::new(probe, validators);
    node.submit(Transaction::new(partner, kind, payload));
    node.try_produce(0, 1_000);
    node
}
