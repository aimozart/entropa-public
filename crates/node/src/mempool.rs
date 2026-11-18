//! The mempool — pending transactions awaiting inclusion in a block.

use entropa_core::Transaction;

/// A FIFO pool of pending transactions.
#[derive(Debug, Default, Clone)]
pub struct Mempool {
    pending: Vec<Transaction>,
}

impl Mempool {
    pub fn new() -> Self {
        Self::default()
    }

    /// Submit a transaction into the pool.
    pub fn submit(&mut self, tx: Transaction) {
        self.pending.push(tx);
    }

    /// Remove and return up to `max` transactions, oldest first.
    pub fn drain(&mut self, max: usize) -> Vec<Transaction> {
        let n = max.min(self.pending.len());
        self.pending.drain(..n).collect()
    }

    pub fn pending(&self) -> &[Transaction] {
        &self.pending
    }

    pub fn len(&self) -> usize {
        self.pending.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}
