use crate::db::Database;
use bincode::encode_to_vec;
use bincode::{Encode, config::standard};
use chrono::{DateTime, Utc};
use ed25519_dalek::ed25519::signature::Verifier;
use ed25519_dalek::{Signature, VerifyingKey};
use libp2p::futures;
use libp2p::futures::lock::Mutex;
use rand::distr::Distribution;
use rand::distr::weighted::WeightedIndex;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use sha2::{Digest, Sha256};
use std::sync::Arc;

pub type Hash = [u8; 32];
pub type Address = [u8; 32];

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct User {
    pub address: Address,
    pub public_key: [u8; 32],
    pub balance: u64,
    pub stake: u64,
}

#[derive(Clone, Debug, Deserialize, Encode, Serialize)]
pub struct Transaction {
    pub sender: Address,
    pub receiver: Address,
    pub amount: u64,
    pub nonce: u64,
    #[serde(with = "BigArray")]
    pub signature: [u8; 64],
    pub sender_public_key: [u8; 32],
}

#[derive(Clone, Debug, Deserialize, Encode, Serialize)]
pub struct Block {
    pub previous_hash: Hash,
    pub merkle_root: Hash,
    pub nonce: u64,
    pub timestamp: i64,
    pub transactions: Vec<Transaction>,
}

pub struct Blockchain {
    db: Arc<Mutex<Database>>,
}

impl Block {
    pub fn new(previous_hash: Hash, nonce: u64, transactions: Vec<Transaction>) -> Self {
        let merkle_root = compute_merkle_root(&transactions);
        Block {
            previous_hash,
            merkle_root,
            nonce,
            timestamp: Utc::now().timestamp(),
            transactions,
        }
    }

    pub fn get_datetime(&self) -> DateTime<Utc> {
        DateTime::from_timestamp(self.timestamp, 0).unwrap_or_else(|| Utc::now())
    }

    pub fn hash(&self) -> Result<Hash, String> {
        let config = standard();
        let encoded =
            encode_to_vec(self, config).map_err(|_| "Error: Failed to encode block".to_string())?;

        let digest = Sha256::digest(&encoded);
        Ok(digest.into())
    }
}

impl Blockchain {
    pub fn new(db: Arc<Mutex<Database>>) -> Self {
        Blockchain { db }
    }

    pub async fn add_block(&mut self, block: Block, proposer: Address) -> Result<(), String> {
        let selected_validator = self.select_validator().await?;
        if proposer != selected_validator {
            return Err("Unauthorized block proposer".to_string());
        }

        let mut db = self.db.lock().await;
        db.add_block(&block)
            .map_err(|_| "Error adding block".to_string())?;
        self.reward_validator(proposer, 10).await?;
        Ok(())
    }

    pub async fn get_block(&self, hash: Hash) -> Result<Option<Block>, rusqlite::Error> {
        let db = self.db.lock().await;
        db.get_block(&hash)
    }

    pub async fn get_blocks(&self) -> Result<Vec<Block>, rusqlite::Error> {
        let db = self.db.lock().await;
        db.get_blocks()
    }

    pub async fn add_transaction(&self, transaction: Transaction) -> Result<(), String> {
        if self.validate_transaction(&transaction) {
            let db = self.db.lock().await;
            db.add_transaction(&transaction)
                .map_err(|_| "Error: Failed to add transaction to the database".to_string())?;
            Ok(())
        } else {
            Err("Error: Invalid transaction".to_string())
        }
    }

    pub async fn get_transactions(&self) -> Result<Vec<Transaction>, rusqlite::Error> {
        let db = self.db.lock().await;
        db.get_transactions()
    }

    pub async fn add_user(&self, user: User) -> Result<(), rusqlite::Error> {
        let db = self.db.lock().await;
        db.add_user(&user)
    }

    pub async fn get_users(&self) -> Result<Vec<User>, rusqlite::Error> {
        let db = self.db.lock().await;
        db.get_users()
    }

    pub async fn select_validator(&self) -> Result<Address, String> {
        let db = self.db.lock().await;
        let users = db
            .get_users()
            .map_err(|_| "Error fetching users".to_string())?;

        let stakes: Vec<u64> = users.iter().map(|user| user.stake).collect();
        let addresses: Vec<Address> = users.iter().map(|user| user.address).collect();

        if stakes.iter().all(|&stake| stake == 0) {
            return Err("No users with stakes available".to_string());
        }

        let dist = WeightedIndex::new(&stakes)
            .map_err(|_| "Error creating weighted distribution".to_string())?;
        let mut rng = rand::rng();
        let selected_index = dist.sample(&mut rng);

        Ok(addresses[selected_index])
    }

    pub async fn stake(&self, user_address: Address, amount: u64) -> Result<(), String> {
        let db = self.db.lock().await;
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
        let db = self.db.lock().await;
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
        let db = self.db.lock().await;
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
        let db = self.db.lock().await;
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

    fn validate_transaction(&self, tx: &Transaction) -> bool {
        let mut hasher = Sha256::new();
        hasher.update(&tx.sender_public_key);
        let computed_address = hasher.finalize();
        if tx.sender != computed_address[..] {
            eprintln!("Invalid address: does not match public key");
            return false;
        }

        if !self.verify_signature(tx) {
            eprintln!("Invalid signature");
            return false;
        }

        let db_guard = match futures::executor::block_on(self.db.lock()) {
            db => db,
        };

        let sender = match db_guard.get_user(&tx.sender) {
            Ok(Some(user)) => user,
            _ => {
                eprintln!("Sender not found");
                return false;
            }
        };

        let expected_nonce = db_guard.get_nonce(&tx.sender).unwrap_or(0);
        if tx.nonce != expected_nonce {
            eprintln!(
                "Invalid nonce: expected {}, got {}",
                expected_nonce, tx.nonce
            );
            return false;
        }

        if sender.balance < tx.amount {
            eprintln!("Insufficient balance");
            return false;
        }

        true
    }

    fn verify_signature(&self, tx: &Transaction) -> bool {
        let verifying_key = match VerifyingKey::from_bytes(&tx.sender_public_key) {
            Ok(pk) => pk,
            Err(_) => return false,
        };

        let signature = Signature::from_bytes(&tx.signature);

        let message = match encode_to_vec(
            (&tx.sender, &tx.receiver, tx.amount, tx.nonce),
            bincode::config::standard(),
        ) {
            Ok(m) => m,
            Err(_) => return false,
        };

        verifying_key.verify(&message, &signature).is_ok()
    }
}

fn compute_merkle_root(transactions: &[Transaction]) -> Hash {
    use sha2::Digest;
    use sha2::Sha256;

    if transactions.is_empty() {
        return Sha256::digest(b"").into();
    }

    let mut hashes: Vec<Hash> = transactions
        .iter()
        .map(|tx| {
            let encoded = bincode::encode_to_vec(tx, bincode::config::standard())
                .expect("failed to encode transaction");
            Sha256::digest(&encoded).into()
        })
        .collect();

    while hashes.len() > 1 {
        if hashes.len() % 2 != 0 {
            hashes.push(hashes.last().unwrap().clone()); // make even
        }

        hashes = hashes
            .chunks(2)
            .map(|pair| {
                let mut hasher = Sha256::new();
                hasher.update(&pair[0]);
                hasher.update(&pair[1]);
                hasher.finalize().into()
            })
            .collect();
    }

    hashes[0]
}
