use crate::{BlockchainError, Result, crypto::Address, transaction::Transaction};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct State {
    balances: HashMap<Address, u64>,
    nonces: HashMap<Address, u64>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_balance(&self, address: &Address) -> u64 {
        *self.balances.get(address).unwrap_or(&0)
    }

    pub fn get_nonce(&self, address: &Address) -> u64 {
        *self.nonces.get(address).unwrap_or(&0)
    }

    pub fn apply_transaction(&mut self, tx: &Transaction) -> Result<()> {
        let current_nonce = self.get_nonce(&tx.sender);
        if tx.nonce != current_nonce + 1 {
            return Err(BlockchainError::InvalidNonce);
        }

        let sender_balance = self.get_balance(&tx.sender);
        if sender_balance < tx.amount {
            return Err(BlockchainError::InsufficientBalance);
        }

        self.balances.insert(tx.sender, sender_balance - tx.amount);
        let receiver_balance = self.get_balance(&tx.receiver);
        self.balances
            .insert(tx.receiver, receiver_balance + tx.amount);

        self.nonces.insert(tx.sender, tx.nonce);

        Ok(())
    }
}
