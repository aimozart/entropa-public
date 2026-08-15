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
    pub fn recover_longest_valid_prefix(mut blocks: Vec<Block>) -> Chain {
        blocks.sort_by_key(|b| b.index);
        let mut chain = Chain::default();
        for block in blocks {
            let expected = chain.blocks.len() as u64;
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
    fn recovers_full_chain_when_no_gap() {
        let (chain, _, _) = sample_chain();
        let recovered = Chain::recover_longest_valid_prefix(chain.blocks.clone());
        assert_eq!(recovered.len(), 3);
        assert_eq!(recovered.verify(), Ok(()));
    }

    #[test]
    fn recovers_longest_valid_prefix_after_gap() {
        // Simulates a single missed Firestore write in the middle of the chain —
        // the exact real-world scenario that used to discard the ENTIRE chain
        // (26,000+ blocks, in production) on the next resume. One missing block
        // must now cost only the blocks after it, not the whole history.
        let (chain, _, _) = sample_chain();
        let mut blocks = chain.blocks.clone();
        blocks.remove(1); // block index 1 never made it to storage
        let recovered = Chain::recover_longest_valid_prefix(blocks);
        // Block 2 can't attach without block 1 present, so only genesis survives —
        // but genesis (real history) is preserved instead of being wiped to zero.
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered.verify(), Ok(()));
    }

    #[test]
    fn recovers_prefix_when_tail_is_corrupted() {
        let (mut chain, _, _) = sample_chain();
        chain.blocks[2].transactions[0].payload = "TAMPERED".into();
        let recovered = Chain::recover_longest_valid_prefix(chain.blocks.clone());
        assert_eq!(recovered.len(), 2);
        assert_eq!(recovered.verify(), Ok(()));
    }

    #[test]
    fn recover_returns_empty_chain_when_genesis_itself_is_bad() {
        let (mut chain, _, _) = sample_chain();
        chain.blocks[0].hash = "0".repeat(64);
        let recovered = Chain::recover_longest_valid_prefix(chain.blocks.clone());
        assert!(recovered.is_empty());
    }

    /// Builds a long, fully valid chain — the scale this needs to hold up at is
    /// tens of thousands of blocks in production, not three.
    fn long_valid_chain(n: usize) -> Chain {
        let founder = Probe::spawn();
        let mut chain = Chain::genesis(&founder, 1_000, beacon::sample(0));
        for i in 1..n {
            chain.propose(
                &founder,
                1_000 + i as u64,
                beacon::sample(i as u64),
                vec![Transaction::new(founder.id(), "attest", format!("round {i}"))],
            );
        }
        chain
    }

    #[test]
    fn recovers_full_1000_block_chain_with_no_corruption() {
        let chain = long_valid_chain(1000);
        let recovered = Chain::recover_longest_valid_prefix(chain.blocks.clone());
        assert_eq!(recovered.len(), 1000);
        assert_eq!(recovered.verify(), Ok(()));
    }

    #[test]
    fn single_missing_block_near_the_end_costs_only_the_tail() {
        // The exact production shape: 999 good blocks, then one silently missing
        // write near the very end (index 999 of 1000).
        let chain = long_valid_chain(1000);
        let mut blocks = chain.blocks.clone();
        blocks.remove(999);
        let recovered = Chain::recover_longest_valid_prefix(blocks);
        assert_eq!(recovered.len(), 999, "must keep all 999 good blocks, not discard everything");
        assert_eq!(recovered.verify(), Ok(()));
    }

    #[test]
    fn single_missing_block_near_the_start_costs_almost_everything() {
        // The other extreme: if the gap is near genesis, most of the chain is
        // unrecoverable by definition (hash-linking requires the predecessor) —
        // this is expected and correct, not a bug. What matters is it's exactly
        // this much loss and no more (not the whole 1000, and not zero).
        let chain = long_valid_chain(1000);
        let mut blocks = chain.blocks.clone();
        blocks.remove(5);
        let recovered = Chain::recover_longest_valid_prefix(blocks);
        assert_eq!(recovered.len(), 5);
        assert_eq!(recovered.verify(), Ok(()));
    }

    #[test]
    fn multiple_gaps_stops_at_the_first_one_only() {
        let chain = long_valid_chain(1000);
        let mut blocks = chain.blocks.clone();
        blocks.remove(700); // second gap — never reached, since the first gap wins
        blocks.remove(300); // first gap encountered during sequential replay
        let recovered = Chain::recover_longest_valid_prefix(blocks);
        assert_eq!(recovered.len(), 300);
        assert_eq!(recovered.verify(), Ok(()));
    }

    #[test]
    fn recovery_is_immune_to_arbitrary_input_order() {
        // Firestore's list API is expected to return blocks pre-sorted by index,
        // but recovery must not silently depend on that — feed it in reverse and
        // in a shuffled order and confirm identical results either way.
        let chain = long_valid_chain(200);
        let mut reversed = chain.blocks.clone();
        reversed.reverse();
        let recovered_reversed = Chain::recover_longest_valid_prefix(reversed);
        assert_eq!(recovered_reversed.len(), 200);
        assert_eq!(recovered_reversed.verify(), Ok(()));

        let mut shuffled = chain.blocks.clone();
        // deterministic "shuffle": interleave odd/even indices
        let (evens, odds): (Vec<_>, Vec<_>) =
            shuffled.drain(..).partition(|b| b.index % 2 == 0);
        let mut interleaved = Vec::new();
        for pair in evens.into_iter().zip(odds.into_iter().chain(std::iter::empty())) {
            interleaved.push(pair.1);
            interleaved.push(pair.0);
        }
        let recovered_shuffled = Chain::recover_longest_valid_prefix(interleaved);
        assert_eq!(recovered_shuffled.len(), 200);
        assert_eq!(recovered_shuffled.verify(), Ok(()));
    }

    #[test]
    fn duplicate_block_at_same_index_does_not_break_recovery() {
        // A retried write landing twice (e.g. a save's retry succeeding after the
        // caller already believed it failed) must not corrupt recovery — the
        // second copy at an already-filled index is simply rejected, same as any
        // other block that doesn't fit next.
        let chain = long_valid_chain(50);
        let mut blocks = chain.blocks.clone();
        let dup = blocks[10].clone();
        blocks.push(dup);
        let recovered = Chain::recover_longest_valid_prefix(blocks);
        assert_eq!(recovered.len(), 50);
        assert_eq!(recovered.verify(), Ok(()));
    }

    #[test]
    fn tampered_transaction_deep_in_a_long_chain_is_caught_and_bounded() {
        let mut chain = long_valid_chain(500);
        chain.blocks[250].transactions[0].payload = "FORGED".into();
        let recovered = Chain::recover_longest_valid_prefix(chain.blocks.clone());
        assert_eq!(recovered.len(), 250);
        assert_eq!(recovered.verify(), Ok(()));
    }

    #[test]
    fn forged_signature_deep_in_a_long_chain_is_caught_and_bounded() {
        let mut chain = long_valid_chain(500);
        let attacker = Probe::spawn();
        let digest = block_digest(
            chain.blocks[300].index,
            chain.blocks[300].timestamp,
            &chain.blocks[300].prev_hash,
            &chain.blocks[300].beacon,
            &chain.blocks[300].transactions,
            &chain.blocks[300].proposer_id,
        );
        chain.blocks[300].signature = attacker.sign_hex(&digest);
        let recovered = Chain::recover_longest_valid_prefix(chain.blocks.clone());
        assert_eq!(recovered.len(), 300);
        assert_eq!(recovered.verify(), Ok(()));
    }

    #[test]
    fn empty_input_recovers_empty_chain_without_panicking() {
        let recovered = Chain::recover_longest_valid_prefix(Vec::new());
        assert!(recovered.is_empty());
    }

    #[test]
    fn rollback_detected_flags_only_when_something_was_actually_discarded() {
        assert!(!rollback_detected(1000, 1000), "no gap: must not flag a rollback");
        assert!(rollback_detected(1000, 999), "any discard at all must be flagged");
        assert!(rollback_detected(1000, 0), "total loss must be flagged");
        assert!(!rollback_detected(0, 0), "nothing loaded, nothing recovered: not a rollback");
    }

    #[test]
    fn bound_to_window_is_a_noop_when_chain_is_already_within_it() {
        let chain = long_valid_chain(50);
        let bounded = chain.clone().bound_to_window(1000);
        assert_eq!(bounded.blocks.len(), 50);
        assert_eq!(bounded.base_index, 0);
        assert_eq!(bounded.height(), 50);
        assert_eq!(bounded.verify(), Ok(()));
    }

    #[test]
    fn bound_to_window_keeps_only_the_most_recent_n_blocks() {
        let chain = long_valid_chain(1000);
        let bounded = chain.bound_to_window(200);
        assert_eq!(bounded.blocks.len(), 200, "in-memory footprint must shrink to the window");
        assert_eq!(bounded.height(), 1000, "logical height must be unaffected by eviction");
        assert_eq!(bounded.blocks.first().unwrap().index, 800);
        assert_eq!(bounded.blocks.last().unwrap().index, 999);
    }

    #[test]
    fn bounded_chain_verifies_against_its_floor_hash_without_full_history() {
        let chain = long_valid_chain(1000);
        let bounded = chain.bound_to_window(200);
        // verify() must succeed using only the 200 retained blocks + the floor hash —
        // it never sees the 800 evicted blocks, proving verification cost is bounded
        // by the window, not by total height.
        assert_eq!(bounded.verify(), Ok(()));
    }

    #[test]
    fn bounded_chain_detects_tamper_within_its_window() {
        let mut chain = long_valid_chain(1000);
        chain.blocks[950].transactions[0].payload = "FORGED".into();
        let bounded = chain.bound_to_window(200);
        assert_eq!(bounded.verify(), Err(ChainError::BadHash(950)));
    }

    #[test]
    fn bounded_chain_continues_producing_correctly_after_eviction() {
        let founder = Probe::spawn();
        let mut chain = Chain::genesis(&founder, 1_000, beacon::sample(0));
        for i in 1..500 {
            chain.propose(&founder, 1_000 + i, beacon::sample(i), Vec::new());
        }
        let mut bounded = chain.bound_to_window(100);
        assert_eq!(bounded.height(), 500);
        // Propose more blocks against the already-bounded chain — must link
        // correctly off the retained tail, not the evicted floor.
        for i in 500..510 {
            bounded.propose(&founder, 1_000 + i, beacon::sample(i), Vec::new());
        }
        assert_eq!(bounded.height(), 510);
        assert_eq!(bounded.verify(), Ok(()));
    }

    #[test]
    fn repeated_eviction_keeps_memory_flat_regardless_of_total_height() {
        // The actual production shape: block production never stops, so this must
        // hold no matter how many total blocks have ever existed — proving the
        // fix for the real O(n)-with-height memory growth found during soak
        // monitoring (Memory limit of 512 MiB exceeded at height ~36,845).
        const WINDOW: usize = 100;
        let founder = Probe::spawn();
        let mut chain = Chain::genesis(&founder, 1_000, beacon::sample(0));
        for i in 1..1_200u64 {
            chain.propose(&founder, 1_000 + i, beacon::sample(i), Vec::new());
            chain = chain.bound_to_window(WINDOW);
            assert!(
                chain.blocks.len() <= WINDOW,
                "in-memory footprint exceeded the window at height {}",
                chain.height()
            );
        }
        assert_eq!(chain.height(), 1_200);
        assert_eq!(chain.verify(), Ok(()));
    }

    #[test]
    fn bounded_chain_try_append_rejects_wrong_index_relative_to_floor() {
        let chain = long_valid_chain(300).bound_to_window(50);
        let rogue = Probe::spawn();
        let mut c = chain.clone();
        let foreign = Chain::default().draft(&rogue, 5, beacon::sample(9), Vec::new());
        assert_eq!(c.try_append(foreign), Err(ChainError::BadIndex(300)));
    }

    #[test]
    fn serde_round_trip() {
        let (chain, _, _) = sample_chain();
        let json = serde_json::to_string(&chain.blocks).unwrap();
        let back: Vec<Block> = serde_json::from_str(&json).unwrap();
        assert_eq!(back.len(), 3);
    }
}
