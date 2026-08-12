//! # Entropa core
//!
//! Post-quantum blockchain primitives for Entropa — *"AI probes reach post-quantum
//! consensus, seeded by the entropy of space."*
//!
//! - [`pqc`] — Probe identities and ML-DSA (FIPS-204) post-quantum signatures.
//! - [`block`] — transactions and blocks, with a canonical blake3 digest.
//! - [`chain`] — an append-only, fully-verifiable constellation of signed blocks.
//! - [`beacon`] — the cosmic entropy beacon that makes `.space` a feature.
//!
//! Everything here is pure Rust and has no networking — [`chain::Chain`] is the ledger
//! the `entropa-node`, `entropa-agents`, and `entropa-api` crates build on.

pub mod beacon;
pub mod block;
pub mod chain;
pub mod pqc;

pub use block::{block_digest, Block, Transaction};
pub use chain::{Chain, ChainError, BIG_BANG};
pub use pqc::{probe_id, verify_hex, Probe};
