use crate::{Result, block::Block, state::State, transaction::Transaction};
use std::fs;
use std::io::{self, BufReader, BufWriter};
use std::path::Path;

pub struct Blockchain {
    pub blocks: Vec<Block>,
    pub state: State,
    pub pending_transactions: Vec<Transaction>,
}

impl Blockchain {
    pub fn new() -> Self {
        let genesis_block = Block::new(vec![], [0; 32]);
        Self {
            blocks: vec![genesis_block],
            state: State::new(),
            pending_transactions: Vec::new(),
        }
    }

    pub fn from_blocks(blocks: Vec<Block>) -> Self {
        let mut state = State::new();
        for block in &blocks {
            for tx in &block.transactions {
                state.apply_transaction(tx).unwrap_or_else(|_| ());
            }
        }
        Self {
            blocks,
            state,
            pending_transactions: Vec::new(),
        }
    }

    pub fn load_blocks_from_db(path: &Path) -> Result<Vec<Block>> {
        if !path.exists() {
            return Ok(vec![Block::new(vec![], [0; 32])]);
        }

        let file = fs::File::open(path).unwrap();
        let reader = BufReader::new(file);
        let blocks = serde_json::from_reader(reader)
            .map_err(|e| e.to_string())
            .unwrap();
        Ok(blocks)
    }

    pub fn save_blocks_to_db(&self, path: &Path) -> io::Result<()> {
        let file = fs::File::create(path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer(writer, &self.blocks)?;
        Ok(())
    }

    pub fn add_transaction(&mut self, transaction: Transaction) -> Result<()> {
        transaction.verify()?;
        self.state.apply_transaction(&transaction)?;
        self.pending_transactions.push(transaction);
        Ok(())
    }

    pub fn mine_block(&mut self) -> Result<Block> {
        let previous_hash = self.blocks.last().map(|b| b.hash).unwrap_or([0; 32]);
        let block = Block::new(self.pending_transactions.drain(..).collect(), previous_hash);
        block.verify()?;
        self.blocks.push(block.clone());
        Ok(block)
    }

    pub fn verify_chain(&self) -> Result<()> {
        for (i, block) in self.blocks.iter().enumerate() {
            block.verify()?;

            if i > 0 {
                let previous_block = &self.blocks[i - 1];
                if block.previous_hash != previous_block.hash {
                    return Err(crate::BlockchainError::InvalidHash);
                }
            }
        }
        Ok(())
    }
}
