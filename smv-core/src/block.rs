use crate::{
    BlockchainError, Result,
    crypto::{Hash, hash},
    transaction::Transaction,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

const DIFFICULTY: usize = 2; // number of leading zeros

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub timestamp: DateTime<Utc>,
    pub transactions: Vec<Transaction>,
    pub previous_hash: Hash,
    pub hash: Hash,
    pub nonce: u64,
}

impl Block {
    pub fn new(transactions: Vec<Transaction>, previous_hash: Hash) -> Self {
        let mut block = Self {
            timestamp: Utc::now(),
            transactions,
            previous_hash,
            hash: [0; 32],
            nonce: 0,
        };
        block.mine();
        block
    }

    pub fn mine(&mut self) {
        while !self.is_valid_proof() {
            self.nonce += 1;
            self.hash = self.calculate_hash();
        }
    }

    pub fn calculate_hash(&self) -> Hash {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.timestamp.timestamp().to_le_bytes());
        for tx in &self.transactions {
            bytes.extend_from_slice(&tx.hash());
        }
        bytes.extend_from_slice(&self.previous_hash);
        bytes.extend_from_slice(&self.nonce.to_le_bytes());
        hash(&bytes)
    }

    fn is_valid_proof(&self) -> bool {
        let hash = self.calculate_hash();
        let required_zeros = vec![0; DIFFICULTY];
        hash.starts_with(&required_zeros)
    }

    pub fn verify(&self) -> Result<()> {
        if !self.is_valid_proof() {
            return Err(BlockchainError::InvalidProofOfWork);
        }

        if self.hash != self.calculate_hash() {
            return Err(BlockchainError::InvalidHash);
        }

        for tx in &self.transactions {
            tx.verify()?;
        }

        Ok(())
    }
}
