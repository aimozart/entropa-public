//! The chain — an append-only, post-quantum-signed constellation of blocks.
//!
//! Genesis is the "Big Bang" block. Each subsequent block is *proposed* by a Probe,
//! which signs it with its ML-DSA key. [`Chain::verify`] re-checks the entire history:
//! sequential indices, matching links, recomputed hashes, and every post-quantum
//! signature. Any tampering — a rewritten transaction, a forged signature — fails it.
//!
//! [`Chain::draft`] builds and signs a block *without* appending (so a proposer can
//! broadcast the exact block it appends); [`Chain::try_append`] validates a block
//! (structure + PQC signature) against the head and appends it. Consensus rules —
//! *who* is allowed to propose in a round — live in the `entropa-node` layer.

use crate::block::{block_digest, Block, Transaction};
use crate::pqc::{probe_id, verify_hex, Probe};

/// The prev-hash of the genesis block.
pub const BIG_BANG: &str = "BIGBANG";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ChainError {
    #[error("block {0}: index out of sequence")]
    BadIndex(u64),
    #[error("block {0}: prev_hash does not link to the previous block")]
    BrokenLink(u64),
    #[error("block {0}: hash does not match recomputed digest")]
    BadHash(u64),
    #[error("block {0}: proposer_id does not match proposer_pubkey")]
    ForgedProposer(u64),
    #[error("block {0}: post-quantum signature is invalid")]
    BadSignature(u64),
    #[error("chain is empty")]
    Empty,
}

/// An append-only chain of post-quantum-signed blocks.
#[derive(Debug, Default, Clone)]
pub struct Chain {
    pub blocks: Vec<Block>,
}

impl Chain {
    /// Create a new chain, proposing the genesis ("Big Bang") block with `founder`.
    pub fn genesis(founder: &Probe, timestamp: u64, beacon: impl Into<String>) -> Self {
        let mut chain = Chain { blocks: Vec::new() };
        chain.propose(founder, timestamp, beacon, Vec::new());
        chain
    }

    /// Build and post-quantum-sign the next block **without appending it**.
    ///
    /// Lets a proposer broadcast the exact block it will append. The block links to the
    /// current head (or `BIG_BANG` if the chain is empty).
    pub fn draft(
        &self,
        proposer: &Probe,
        timestamp: u64,
        beacon: impl Into<String>,
        transactions: Vec<Transaction>,
    ) -> Block {
        let index = self.blocks.len() as u64;
        let prev_hash = self
            .blocks
            .last()
            .map(|b| b.hash.clone())
            .unwrap_or_else(|| BIG_BANG.to_string());
        let beacon = beacon.into();
        let proposer_id = proposer.id();
        let proposer_pubkey = proposer.pubkey_hex();
        let digest = block_digest(
            index,
            timestamp,
            &prev_hash,
            &beacon,
            &transactions,
            &proposer_id,
        );
        let signature = proposer.sign_hex(&digest);
        Block {
            index,
            timestamp,
            prev_hash,
            beacon,
            transactions,
            proposer_id,
            proposer_pubkey,
            hash: hex::encode(digest),
            signature,
        }
    }

    /// Validate a block against the current head (index, link, hash, proposer
    /// fingerprint, PQC signature) and append it. Does **not** enforce consensus rules
    /// (who may propose) — that is the node layer's job.
    pub fn try_append(&mut self, block: Block) -> Result<(), ChainError> {
        let expected = self.blocks.len() as u64;
        let prev = self
            .blocks
            .last()
            .map(|b| b.hash.as_str())
            .unwrap_or(BIG_BANG);
        if block.index != expected {
            return Err(ChainError::BadIndex(expected));
        }
        if block.prev_hash != prev {
            return Err(ChainError::BrokenLink(expected));
        }
        if probe_id(&block.proposer_pubkey) != block.proposer_id {
            return Err(ChainError::ForgedProposer(expected));
        }
        let digest = block_digest(
            block.index,
            block.timestamp,
            &block.prev_hash,
            &block.beacon,
            &block.transactions,
            &block.proposer_id,
        );
        if hex::encode(digest) != block.hash {
            return Err(ChainError::BadHash(expected));
        }
        if !verify_hex(&block.proposer_pubkey, &digest, &block.signature) {
            return Err(ChainError::BadSignature(expected));
        }
        self.blocks.push(block);
        Ok(())
    }

    /// Propose (draft + append) the next block with `proposer`. Returns its index.
    pub fn propose(
        &mut self,
        proposer: &Probe,
        timestamp: u64,
        beacon: impl Into<String>,
        transactions: Vec<Transaction>,
    ) -> u64 {
        let block = self.draft(proposer, timestamp, beacon, transactions);
        let index = block.index;
        self.try_append(block)
            .expect("a self-drafted block is always valid");
        index
    }

    /// Verify the entire chain: links, hashes, and every post-quantum signature.
    pub fn verify(&self) -> Result<(), ChainError> {
        if self.blocks.is_empty() {
            return Err(ChainError::Empty);
        }
        let mut prev = BIG_BANG.to_string();
        for (i, b) in self.blocks.iter().enumerate() {
            let i = i as u64;
            if b.index != i {
                return Err(ChainError::BadIndex(i));
            }
            if b.prev_hash != prev {
                return Err(ChainError::BrokenLink(i));
            }
            if probe_id(&b.proposer_pubkey) != b.proposer_id {
                return Err(ChainError::ForgedProposer(i));
            }
            let digest = block_digest(
                b.index,
                b.timestamp,
                &b.prev_hash,
                &b.beacon,
                &b.transactions,
                &b.proposer_id,
            );
            if hex::encode(digest) != b.hash {
                return Err(ChainError::BadHash(i));
            }
            if !verify_hex(&b.proposer_pubkey, &digest, &b.signature) {
                return Err(ChainError::BadSignature(i));
            }
            prev = b.hash.clone();
        }
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    pub fn head(&self) -> Option<&Block> {
        self.blocks.last()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::beacon;

    fn sample_chain() -> (Chain, Probe, Probe) {
        let founder = Probe::spawn();
        let alice = Probe::spawn();
        let mut chain = Chain::genesis(&founder, 1_000, beacon::sample(0));
        chain.propose(
            &alice,
            1_001,
            beacon::sample(1),
            vec![Transaction::new(
                alice.id(),
                "transfer",
                "10 -> PROBE-DEADBEEF",
            )],
        );
        chain.propose(
            &founder,
            1_002,
            beacon::sample(2),
            vec![Transaction::new(
                "oracle",
                "attest",
                "cosmic beacon round 2",
            )],
        );
        (chain, founder, alice)
    }

    #[test]
    fn builds_and_verifies() {
        let (chain, _, _) = sample_chain();
        assert_eq!(chain.len(), 3);
        assert_eq!(chain.verify(), Ok(()));
    }

    #[test]
    fn detects_tampered_transaction() {
        let (mut chain, _, _) = sample_chain();
        chain.blocks[1].transactions[0].payload = "9000 -> PROBE-ATTACKER".into();
        assert_eq!(chain.verify(), Err(ChainError::BadHash(1)));
    }

    #[test]
    fn detects_forged_signature() {
        let (mut chain, _, _) = sample_chain();
        let attacker = Probe::spawn();
        let digest = block_digest(
            chain.blocks[2].index,
            chain.blocks[2].timestamp,
            &chain.blocks[2].prev_hash,
            &chain.blocks[2].beacon,
            &chain.blocks[2].transactions,
            &chain.blocks[2].proposer_id,
        );
        chain.blocks[2].signature = attacker.sign_hex(&digest);
        assert_eq!(chain.verify(), Err(ChainError::BadSignature(2)));
    }

    #[test]
    fn detects_broken_link() {
        let (mut chain, _, _) = sample_chain();
        chain.blocks[1].prev_hash = "0".repeat(64);
        assert_eq!(chain.verify(), Err(ChainError::BrokenLink(1)));
    }

    #[test]
    fn try_append_rejects_foreign_block() {
        // A block drafted against a different (empty) chain won't link to a 3-block head.
        let (mut chain, _, _) = sample_chain();
        let rogue = Probe::spawn();
        let foreign = Chain::default().draft(&rogue, 5, beacon::sample(9), Vec::new());
        assert_eq!(chain.try_append(foreign), Err(ChainError::BadIndex(3)));
    }

    #[test]
    fn serde_round_trip() {
        let (chain, _, _) = sample_chain();
        let json = serde_json::to_string(&chain.blocks).unwrap();
        let back: Vec<Block> = serde_json::from_str(&json).unwrap();
        assert_eq!(back.len(), 3);
    }
}
