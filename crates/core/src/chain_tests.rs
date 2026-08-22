//! Unit tests for `chain.rs` — kept in a separate file (via `#[path]` in `chain.rs`)
//! purely to keep both files under the L20 500-line ceiling. This is still the same
//! `chain::tests` module, same visibility, same `cargo test` output — file location
//! is the only thing that changed, not the module tree.

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
            vec![Transaction::new(
                founder.id(),
                "attest",
                format!("round {i}"),
            )],
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
    assert_eq!(
        recovered.len(),
        999,
        "must keep all 999 good blocks, not discard everything"
    );
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
    let (evens, odds): (Vec<_>, Vec<_>) = shuffled.drain(..).partition(|b| b.index % 2 == 0);
    let mut interleaved = Vec::new();
    for pair in evens
        .into_iter()
        .zip(odds.into_iter().chain(std::iter::empty()))
    {
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
    assert!(
        !rollback_detected(1000, 1000),
        "no gap: must not flag a rollback"
    );
    assert!(
        rollback_detected(1000, 999),
        "any discard at all must be flagged"
    );
    assert!(rollback_detected(1000, 0), "total loss must be flagged");
    assert!(
        !rollback_detected(0, 0),
        "nothing loaded, nothing recovered: not a rollback"
    );
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
    assert_eq!(
        bounded.blocks.len(),
        200,
        "in-memory footprint must shrink to the window"
    );
    assert_eq!(
        bounded.height(),
        1000,
        "logical height must be unaffected by eviction"
    );
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
#[ignore = "slow (~minutes, ML-DSA signing dominates): exercises the real production \
            IN_MEMORY_WINDOW (20,000, see crates/api/src/main.rs) at scale rather than a toy \
            window. Run explicitly with `cargo test -- --ignored` before any change to the \
            bounding logic or the window constant, not on every `cargo test --workspace`."]
fn repeated_eviction_stays_flat_at_the_real_production_window() {
    // Same property as repeated_eviction_keeps_memory_flat_regardless_of_total_height, but
    // at the actual deployed window size instead of a toy value — this is the direct forecast
    // of the real production configuration's memory behavior at scale, not just the algorithm's
    // shape. Overshoots the window by 10% (22,000 total) rather than repeating the generic
    // test's 12x ratio, since that ratio is already proven window-size-independent above.
    const WINDOW: usize = 20_000;
    let founder = Probe::spawn();
    let mut chain = Chain::genesis(&founder, 1_000, beacon::sample(0));
    for i in 1..22_000u64 {
        chain.propose(&founder, 1_000 + i, beacon::sample(i), Vec::new());
        chain = chain.bound_to_window(WINDOW);
        assert!(
            chain.blocks.len() <= WINDOW,
            "in-memory footprint exceeded the real production window at height {}",
            chain.height()
        );
    }
    assert_eq!(chain.height(), 22_000);
    assert_eq!(chain.blocks.len(), WINDOW);
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
