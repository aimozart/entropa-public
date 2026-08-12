//! The node — one participant that holds a chain, a mempool, and a validator set, and
//! runs the produce/accept consensus loop.
//!
//! Each round is seeded by the public randomness beacon. If this node is the beacon-
//! selected proposer, it drains its mempool and produces the next block. Blocks from
//! peers are accepted only if (a) the proposer is the correct beacon-selected validator
//! for that block's beacon, and (b) the block passes full structural + post-quantum
//! validation against the chain head.

use entropa_core::{beacon, Block, Chain, ChainError, Probe, Transaction};

use crate::consensus::{select_proposer, Validator};
use crate::mempool::Mempool;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum NodeError {
    #[error("no validators configured")]
    NoValidators,
    #[error("block proposer is not the beacon-selected validator for this round")]
    WrongProposer,
    #[error(transparent)]
    Invalid(#[from] ChainError),
}

/// A network participant.
pub struct Node {
    /// This node's post-quantum identity.
    pub probe: Probe,
    /// The validator set (shared, agreed configuration).
    pub validators: Vec<Validator>,
    /// This node's view of the chain.
    pub chain: Chain,
    /// Pending transactions.
    pub mempool: Mempool,
    /// Max transactions per block.
    pub max_block_txs: usize,
    /// A live beacon value fetched externally for the *current* round, if any (see
    /// `entropa_core::beacon::sample_live`). Set this once per round before calling
    /// `is_proposer`/`try_produce` so both see the identical value — fetching live
    /// separately in each would risk straddling a beacon update between the two calls.
    /// `None` falls back to the deterministic `beacon::sample(round)`.
    pub live_beacon: Option<String>,
}

impl Node {
    pub fn new(probe: Probe, validators: Vec<Validator>) -> Self {
        Self {
            probe,
            validators,
            chain: Chain::default(),
            mempool: Mempool::new(),
            max_block_txs: 64,
            live_beacon: None,
        }
    }

    /// Submit a transaction into this node's mempool.
    pub fn submit(&mut self, tx: Transaction) {
        self.mempool.submit(tx);
    }

    /// The beacon value for `round`: the injected live value if set, else the
    /// deterministic offline fallback.
    fn beacon_for(&self, round: u64) -> String {
        self.live_beacon
            .clone()
            .unwrap_or_else(|| beacon::sample(round))
    }

    /// Am I the beacon-selected proposer for `round`?
    pub fn is_proposer(&self, round: u64) -> bool {
        let b = self.beacon_for(round);
        select_proposer(&self.validators, &b)
            .map(|i| self.validators[i].id == self.probe.id())
            .unwrap_or(false)
    }

    /// If this node is the beacon-selected proposer for `round`, drain the mempool,
    /// produce and append the next block, and return it for broadcast. Otherwise `None`.
    pub fn try_produce(&mut self, round: u64, timestamp: u64) -> Option<Block> {
        if !self.is_proposer(round) {
            return None;
        }
        let b = self.beacon_for(round);
        let txs = self.mempool.drain(self.max_block_txs);
        let block = self.chain.draft(&self.probe, timestamp, b, txs);
        self.chain
            .try_append(block.clone())
            .expect("self-drafted block is valid");
        Some(block)
    }

    /// Accept a peer's block: enforce the consensus rule (correct proposer for the
    /// block's beacon), then full structural + post-quantum validation.
    pub fn accept(&mut self, block: Block) -> Result<(), NodeError> {
        let idx =
            select_proposer(&self.validators, &block.beacon).ok_or(NodeError::NoValidators)?;
        if self.validators[idx].id != block.proposer_id {
            return Err(NodeError::WrongProposer);
        }
        self.chain.try_append(block)?;
        Ok(())
    }

    pub fn height(&self) -> usize {
        self.chain.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build N nodes that share one validator set.
    fn network(n: usize) -> Vec<Node> {
        let probes: Vec<Probe> = (0..n).map(|_| Probe::spawn()).collect();
        let validators: Vec<Validator> = probes
            .iter()
            .map(|p| Validator::new(p.id(), p.pubkey_hex()))
            .collect();
        probes
            .into_iter()
            .map(|p| Node::new(p, validators.clone()))
            .collect()
    }

    #[test]
    fn multi_node_consensus_agrees() {
        let mut nodes = network(3);
        // Give every node the same pending transaction each round.
        for round in 0..6u64 {
            for node in nodes.iter_mut() {
                node.submit(Transaction::new(
                    "user",
                    "transfer",
                    format!("round {round}"),
                ));
            }
            let b = beacon::sample(round);
            let sel = select_proposer(&nodes[0].validators, &b).unwrap();
            let block = nodes[sel]
                .try_produce(round, 1_000 + round)
                .expect("the selected node produces");
            for (i, node) in nodes.iter_mut().enumerate() {
                if i != sel {
                    node.accept(block.clone()).expect("peers accept");
                }
            }
        }
        // All nodes agree on height and each chain fully verifies.
        let heights: Vec<usize> = nodes.iter().map(|n| n.height()).collect();
        assert_eq!(heights, vec![6, 6, 6]);
        let heads: Vec<String> = nodes
            .iter()
            .map(|n| n.chain.head().unwrap().hash.clone())
            .collect();
        assert!(heads.iter().all(|h| h == &heads[0]));
        for node in &nodes {
            assert_eq!(node.chain.verify(), Ok(()));
        }
    }

    #[test]
    fn rejects_block_from_wrong_proposer() {
        let mut nodes = network(3);
        let round = 0u64;
        let b = beacon::sample(round);
        let sel = select_proposer(&nodes[0].validators, &b).unwrap();
        let wrong = (sel + 1) % 3;
        // The wrong node self-drafts a block for this round (bypassing is_proposer).
        let bad = nodes[wrong]
            .chain
            .draft(&nodes[wrong].probe, 42, b, Vec::new());
        // A different node must reject it: wrong proposer for this beacon.
        let victim = (sel + 2) % 3;
        assert_eq!(nodes[victim].accept(bad), Err(NodeError::WrongProposer));
    }

    #[test]
    fn only_one_proposer_per_round() {
        let nodes = network(4);
        for round in 0..12u64 {
            let count = nodes.iter().filter(|n| n.is_proposer(round)).count();
            assert_eq!(count, 1, "exactly one proposer selected per round");
        }
    }
}
