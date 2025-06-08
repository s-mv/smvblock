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
        self.nonces.get(address).unwrap_or(&0) + 1
    }

    pub fn apply_transaction(&mut self, tx: &Transaction) -> Result<()> {
        let expected_nonce = self.get_nonce(&tx.sender);
        if tx.nonce != expected_nonce {
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

    /* todo, later remove these or make them private and find a better way */

    pub fn set_nonce(&mut self, sender_address: &Address, nonce: u64) {
        self.nonces.insert(*sender_address, nonce);
    }

    pub fn set_balance(&mut self, sender_address: &Address, balance: u64) {
        self.balances.insert(*sender_address, balance);
    }
}
