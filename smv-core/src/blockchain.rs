use crate::{
    block::Block,
    state::State,
    transaction::Transaction,
    Result,
};

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
