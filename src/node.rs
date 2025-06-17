use crate::blockchain::{Address, Block, Blockchain, Transfer, User};
use crate::db::Database;
use crate::p2p::P2P;
use ed25519_dalek::SigningKey;
use libp2p::futures::lock::Mutex;
use sha2::{Digest, Sha256};
use std::sync::Arc;

#[derive(Debug)]
pub enum NodeType {
    FullNode,
    LightNode,
}

#[derive(Debug)]
pub struct Node {
    pub node_type: NodeType,
    pub blockchain: Blockchain,
    pub p2p: P2P,
    pub database: Arc<Mutex<Database>>,
}

impl Node {
    pub fn new(node_type: NodeType, test_node: bool) -> Result<Self, String> {
        let database = Database::new(None, test_node)
            .map_err(|_| "Failed to initialize database".to_string())?;
        let database = Arc::new(Mutex::new(database));

        let blockchain = Blockchain::new(database.clone());
        let p2p = P2P::new(database.clone());

        Ok(Node {
            node_type,
            blockchain,
            p2p,
            database,
        })
    }

    pub async fn add_user(&self, user: User) -> Result<(), rusqlite::Error> {
        let db = self.database.lock().await;
        db.add_user(&user)
    }

    pub async fn get_users(&self) -> Result<Vec<User>, rusqlite::Error> {
        let db = self.database.lock().await;
        db.get_users()
    }

    pub async fn stake(&self, user_address: Address, amount: u64) -> Result<(), String> {
        let db = self.database.lock().await;
        let user = db
            .get_user(&user_address)
            .map_err(|_| "Error fetching user".to_string())?;

        if let Some(mut user) = user {
            if user.balance < amount {
                return Err("Insufficient balance to stake".to_string());
            }

            user.balance -= amount;
            user.stake += amount;

            db.update_user(&user)
                .map_err(|_| "Error updating user".to_string())?;
            Ok(())
        } else {
            Err("User not found".to_string())
        }
    }

    pub async fn unstake(&self, user_address: Address, amount: u64) -> Result<(), String> {
        let db = self.database.lock().await;
        let user = db
            .get_user(&user_address)
            .map_err(|_| "Error fetching user".to_string())?;

        if let Some(mut user) = user {
            if user.stake < amount {
                return Err("Insufficient stake to unstake".to_string());
            }

            user.stake -= amount;
            user.balance += amount;

            db.update_user(&user)
                .map_err(|_| "Error updating user".to_string())?;
            Ok(())
        } else {
            Err("User not found".to_string())
        }
    }

    pub async fn reward_validator(
        &self,
        validator_address: Address,
        reward: u64,
    ) -> Result<(), String> {
        let db = self.database.lock().await;
        let user = db
            .get_user(&validator_address)
            .map_err(|_| "Error fetching user".to_string())?;

        if let Some(mut user) = user {
            user.balance += reward;
            db.update_user(&user)
                .map_err(|_| "Error updating user".to_string())?;
            Ok(())
        } else {
            Err("Validator not found".to_string())
        }
    }

    pub async fn slash_validator(
        &self,
        validator_address: Address,
        penalty: u64,
    ) -> Result<(), String> {
        let db = self.database.lock().await;
        let user = db
            .get_user(&validator_address)
            .map_err(|_| "Error fetching user".to_string())?;

        if let Some(mut user) = user {
            if user.stake < penalty {
                user.stake = 0;
            } else {
                user.stake -= penalty;
            }

            db.update_user(&user)
                .map_err(|_| "Error updating user".to_string())?;
            Ok(())
        } else {
            Err("Validator not found".to_string())
        }
    }

    pub async fn send_transaction(
        &self,
        sender_private_key: SigningKey,
        receiver: Address,
        amount: u64,
    ) -> Result<(), String> {
        let db = self.database.lock().await;

        let sender_public_key = crate::blockchain::derive_public_key(&sender_private_key);
        let sender_address: [u8; 32] = Sha256::digest(sender_public_key).into();

        if sender_address == receiver {
            return Err("Sender and receiver cannot be the same".to_string());
        }

        let sender = db
            .get_user(&sender_address)
            .map_err(|_| "Sender not found".to_string())?
            .ok_or("Sender not found".to_string())?;

        if sender.balance < amount {
            return Err("Insufficient balance".to_string());
        }

        let nonce = db.get_latest_nonce(&sender_address).unwrap();

        let transfer = Transfer {
            receiver,
            amount,
            nonce,
        };

        let tx = transfer.into_transaction(&sender_private_key);

        drop(db);
        self.blockchain.add_transaction(tx).await
    }

    pub async fn produce_block(&mut self) -> Result<[u8; 32], String> {
        let transactions = self
            .blockchain
            .get_transactions()
            .await
            .map_err(|_| "Failed to fetch transactions".to_string())?;

        let blocks = self
            .blockchain
            .get_blocks()
            .await
            .map_err(|_| "Failed to fetch blocks".to_string())?;

        let previous_hash = blocks
            .last()
            .map(|b| b.hash().unwrap_or([0u8; 32]))
            .unwrap_or([0u8; 32]);

        let nonce = blocks.len() as u64;
        let proposer = self.blockchain.select_validator().await?;
        let block = Block::new(previous_hash, nonce, transactions);

        self.blockchain.apply_block(&block).await?;
        self.blockchain.add_block(block.clone(), proposer).await?;

        let hash = block
            .hash()
            .map_err(|e| format!("Block hashing failed: {}", e))?;
        println!(
            "Block successfully produced with hash: {}",
            hex::encode(hash)
        );
        Ok(hash)
    }
}
