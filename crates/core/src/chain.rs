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
///
/// `blocks` need not hold the *entire* history — see [`Chain::bound_to_window`].
/// `base_index`/`base_hash` record where the in-memory window starts when older
/// history has been trimmed: `base_index` is the index of the first block no
/// longer held, and `base_hash` is that block's hash (the floor the current
/// window's first block must link to). Both are `0`/empty for a chain that has
/// never evicted anything, in which case the floor is genesis (`BIG_BANG`).
#[derive(Debug, Default, Clone)]
pub struct Chain {
    pub blocks: Vec<Block>,
    pub base_index: u64,
    pub base_hash: String,
}

impl Chain {
    /// Create a new chain, proposing the genesis ("Big Bang") block with `founder`.
    pub fn genesis(founder: &Probe, timestamp: u64, beacon: impl Into<String>) -> Self {
        let mut chain = Chain::default();
        chain.propose(founder, timestamp, beacon, Vec::new());
        chain
    }

    /// The floor hash this window's first block must link to: genesis if nothing has
    /// ever been evicted, otherwise the hash of the last evicted block.
    fn floor_hash(&self) -> &str {
        if self.base_index == 0 {
            BIG_BANG
        } else {
            self.base_hash.as_str()
        }
    }

    /// Total chain height, including any history trimmed out of `blocks` by
    /// [`Chain::bound_to_window`]. Use this instead of `.len()` wherever the real
    /// logical height is needed (e.g. the next block's index) — `.len()` only
    /// reflects what's currently held in memory.
    pub fn height(&self) -> u64 {
        self.base_index + self.blocks.len() as u64
    }

    /// Trim this chain down to at most its `window` most recent blocks. Pure and
    /// deterministic — same input, same output, no I/O, no mutation of anything
    /// outside `self` — a no-op when the chain is already within the window.
    ///
    /// The evicted blocks are not lost, only no longer held in memory: this project
    /// treats Firestore as the durable source of truth for everything below
    /// `base_index`, so `verify()` can still confirm the retained window links
    /// correctly to the history it no longer holds, without re-walking that
    /// history. This is the fix for the O(n)-with-height memory (and, via
    /// `verify()`'s full walk, time) growth found during soak monitoring at height
    /// ~36,845 — see `SESSION_STATE.md` § Bounded-memory chain architecture.
    pub fn bound_to_window(mut self, window: usize) -> Chain {
        if self.blocks.len() <= window {
            return self;
        }
        let cut = self.blocks.len() - window;
        let new_floor = self.blocks[cut - 1].clone();
        self.base_index = new_floor.index + 1;
        self.base_hash = new_floor.hash;
        self.blocks.drain(0..cut);
        self
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
        let index = self.height();
        let prev_hash = self
            .blocks
            .last()
            .map(|b| b.hash.clone())
            .unwrap_or_else(|| self.floor_hash().to_string());
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
        let expected = self.height();
        let prev = self
            .blocks
            .last()
            .map(|b| b.hash.as_str())
            .unwrap_or_else(|| self.floor_hash());
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
        let mut prev = self.floor_hash().to_string();
        for (offset, b) in self.blocks.iter().enumerate() {
            let i = self.base_index + offset as u64;
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

    /// Recover the longest valid prefix from a possibly gappy or corrupted set of
    /// blocks — the situation after loading from storage where a single missed or
    /// malformed write left a hole in the history.
    ///
    /// Blocks are sorted by index, then verified sequentially from genesis via the
    /// same checks [`Chain::try_append`] enforces (contiguous index, linked hash,
    /// recomputed digest, valid PQC signature). Verification stops at the first
    /// block that fails any check; everything before that point is kept.
    ///
    /// This replaces all-or-nothing verification, where a single missing block
    /// anywhere in tens of thousands discarded the *entire* chain back to zero on
    /// resume. Now a gap costs only the blocks after it, not the real history
    /// before it.
    pub fn recover_longest_valid_prefix(blocks: Vec<Block>) -> Chain {
        Self::recover_longest_valid_prefix_from(blocks, 0, String::new())
    }

    /// Same recovery as [`Chain::recover_longest_valid_prefix`], but starting from
    /// a caller-supplied floor instead of always assuming genesis — the same trust
    /// boundary [`Chain::bound_to_window`] already uses for the in-memory case
    /// (this project treats Firestore as the durable source of truth for
    /// everything below `base_index`), extended to resume: a caller can fetch and
    /// validate just the most recent `window` blocks, not the entire history,
    /// bounding resume time/reads to O(window) instead of O(full height). This is
    /// the fix for a real production incident: at height ~176,712 a full-history
    /// resume exceeded Cloud Run's startup probe timeout.
    pub fn recover_longest_valid_prefix_from(
        mut blocks: Vec<Block>,
        base_index: u64,
        base_hash: String,
    ) -> Chain {
        blocks.sort_by_key(|b| b.index);
        let mut chain = Chain {
            blocks: Vec::new(),
            base_index,
            base_hash,
        };
        for block in blocks {
            let expected = chain.height();
            if block.index < expected {
                // A stale duplicate of an index we've already accepted (e.g. a
                // retried write that landed twice) — not a gap, just skip it.
                continue;
            }
            if chain.try_append(block).is_err() {
                break;
            }
        }
        chain
    }
}

/// Pure policy decision, kept separate from I/O so it's directly testable: did
/// recovery have to discard anything? If so, the caller must never keep writing
/// into the same storage namespace the discarded data lived in — see
/// `persistence.rs`'s epoch rotation, which is the actual guarantee that a future
/// gap can only ever cost the *current* epoch's un-recovered tail, never a past
/// epoch's history.
pub fn rollback_detected(loaded_count: usize, recovered_len: usize) -> bool {
    recovered_len < loaded_count
}

#[cfg(test)]
#[path = "chain_tests.rs"]
mod tests;
