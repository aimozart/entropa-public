//! # Entropa core
//!
//! Post-quantum blockchain primitives for Entropa — *"AI probes reach post-quantum
//! consensus, seeded by a public randomness beacon."*
//!
//! - [`pqc`] — Probe identities and ML-DSA (FIPS-204) post-quantum signatures.
//! - [`block`] — transactions and blocks, with a canonical blake3 digest.
//! - [`chain`] — an append-only, fully-verifiable constellation of signed blocks.
//! - [`beacon`] — the public randomness beacon ([drand](https://drand.love)) that
//!   seeds proposer selection; [`chain::Chain`] itself stays pure and offline.
//!
//! [`chain::Chain`] is the ledger the `entropa-node`, `entropa-agents`, and
//! `entropa-api` crates build on. The one piece of this crate that reaches the network
//! is [`beacon::sample_live`] — everything else is pure Rust.

pub mod beacon;
pub mod block;
pub mod chain;
pub mod pqc;

pub use block::{block_digest, Block, Transaction};
pub use chain::{Chain, ChainError, BIG_BANG};
pub use pqc::{probe_id, verify_hex, Probe};
