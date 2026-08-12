//! # Entropa node
//!
//! The consensus engine: a [`Mempool`] of pending transactions, cosmic-seeded proposer
//! selection ([`select_proposer`]), and a [`Node`] that produces and validates blocks.
//!
//! Networking (libp2p gossip, peer discovery) will wrap this crate — the consensus core
//! is deliberately transport-free so it stays fully testable.

pub mod consensus;
pub mod mempool;
pub mod node;

pub use consensus::{select_proposer, Validator};
pub use mempool::Mempool;
pub use node::{Node, NodeError};
