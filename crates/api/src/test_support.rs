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
