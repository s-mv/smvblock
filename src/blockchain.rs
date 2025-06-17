use crate::db::Database;
use bincode::config::standard;
use bincode::{Decode, Encode, encode_to_vec};
use chrono::{DateTime, Utc};
use ed25519_dalek::ed25519::signature::{SignerMut, Verifier};
use ed25519_dalek::{Signature, SigningKey, VerifyingKey};
use libp2p::futures::lock::Mutex;
use rand::distributions::WeightedIndex;
use rand::prelude::Distribution;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;

pub type Hash = [u8; 32];
pub type Address = [u8; 32];

#[derive(Clone, Debug, Deserialize, Serialize, Encode, Decode, PartialEq)]
pub struct Transfer {
    pub receiver: Address,
    pub amount: u64,
    pub nonce: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, Encode, Decode, PartialEq)]
pub struct Transaction {
    pub payload: Transfer,
    pub sender_public_key: [u8; 32],
    #[serde(with = "serde_big_array::BigArray")]
    pub signature: [u8; 64],
}

impl Transfer {
    pub fn serialize(&self) -> Vec<u8> {
        encode_to_vec(self, standard()).expect("Failed to serialize unsigned transaction")
    }

    pub(crate) fn hash(&self) -> [u8; 32] {
        let encoded = self.serialize();
        let digest = Sha256::digest(&encoded);
        digest.into()
    }

    pub fn into_transaction(self, key: &SigningKey) -> Transaction {
        Transaction::sign(self, &mut key.clone())
    }
}

impl Transaction {
    pub fn sign(unsigned: Transfer, signing_key: &mut SigningKey) -> Self {
        let message_hash = unsigned.hash();
        let signature = signing_key.sign(&message_hash);

        Self {
            payload: unsigned,
            sender_public_key: signing_key.verifying_key().to_bytes(),
            signature: signature.to_bytes(),
        }
    }

    pub fn verify(&self) -> bool {
        let verifying_key = match VerifyingKey::from_bytes(&self.sender_public_key) {
            Ok(key) => key,
            Err(_) => return false,
        };

        let message_hash = self.payload.hash();
        let signature = Signature::from_bytes(&self.signature);

        verifying_key.verify(&message_hash, &signature).is_ok()
    }

    pub fn sender_address(&self) -> Address {
        let mut hasher = Sha256::new();
        hasher.update(&self.sender_public_key);
        hasher.finalize().into()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct User {
    pub address: Address,
    pub public_key: [u8; 32],
    pub balance: u64,
    pub stake: u64,
}

#[derive(Clone, Debug, Deserialize, Encode, Serialize)]
pub struct Block {
    pub previous_hash: Hash,
    pub merkle_root: Hash,
    pub nonce: u64,
    pub timestamp: i64,
    pub transactions: Vec<Transaction>,
}

#[derive(Debug)]
pub struct Blockchain {
    db: Arc<Mutex<Database>>,
}

impl User {
    pub fn generate(initial_balance: u64) -> (Self, SigningKey) {
        let mut csprng = OsRng;
        let private_key = SigningKey::generate(&mut csprng);
        let verifying_key = private_key.verifying_key();

        let mut hasher = Sha256::new();
        hasher.update(verifying_key.to_bytes());
        let address = hasher.finalize();

        let user = User {
            address: address.into(),
            public_key: verifying_key.to_bytes(),
            balance: initial_balance,
            stake: 0,
        };

        (user, private_key)
    }
}

pub fn derive_public_key(private_key: &SigningKey) -> VerifyingKey {
    private_key.verifying_key()
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

    pub async fn create_genesis_block(&self) -> Result<(), String> {
        let mut db = self.db.lock().await;

        if db.get_latest_block().map_err(|_| "DB error")?.is_some() {
            return Err("Genesis block already exists".to_string());
        }

        let genesis_block = Block::new([0u8; 32], 0, vec![]);

        db.add_block(&genesis_block)
            .map_err(|_| "Failed to add genesis block".to_string())?;

        println!("Genesis block created!");
        Ok(())
    }

    pub async fn add_block(&mut self, block: Block, proposer: Address) -> Result<(), String> {
        let is_registered = {
            let db = self.db.lock().await;
            db.get_user(&proposer)
                .map_err(|_| "DB error".to_string())?
                .is_some()
        };

        if !is_registered {
            return Err("Proposer not found".to_string());
        }

        for tx in &block.transactions {
            if !tx.verify() {
                return Err("Invalid transaction in block".to_string());
            }
        }

        let mut db = self.db.lock().await;
        db.add_block(&block)
            .map_err(|_| "Error adding block".to_string())?;
        drop(db);

        self.reward_validator(proposer).await?;

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
        let db = self.db.lock().await;
        db.add_transaction(&transaction, transaction.verify())
            .map_err(|_| "Error: Failed to add transaction to the database".to_string())?;
        Ok(())
    }

    pub async fn get_transactions(&self) -> Result<Vec<Transaction>, rusqlite::Error> {
        let db = self.db.lock().await;
        db.get_all_transactions()
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
        let mut rng = rand::thread_rng();
        let selected_index = dist.sample(&mut rng);

        Ok(addresses[selected_index])
    }

    pub async fn reward_validator(&self, validator_address: Address) -> Result<(), String> {
        let db = self.db.lock().await;

        let user = db
            .get_user(&validator_address)
            .map_err(|_| "Error fetching user".to_string())?;

        if let Some(mut user) = user {
            let total_stake = db.get_total_stake().unwrap() as f64;
            let reward = (user.stake as f64 / total_stake).round() as u64;
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

    pub async fn apply_block(&self, block: &Block) -> Result<(), String> {
        let db = self.db.lock().await;

        for tx in &block.transactions {
            let Transfer {
                receiver, amount, ..
            } = tx.payload;
            let sender_pub_key = tx.sender_public_key;
            let sender_address: Address = Sha256::digest(sender_pub_key).into();

            let mut sender = db
                .get_user(&sender_address)
                .map_err(|_| "Sender not found".to_string())?
                .ok_or("Sender not found".to_string())?;

            let mut receiver = db
                .get_user(&receiver)
                .map_err(|_| "Receiver not found".to_string())?
                .ok_or("Receiver not found".to_string())?;

            if sender.balance < amount {
                return Err(format!(
                    "Sender {} has insufficient balance",
                    hex::encode(sender.address)
                ));
            }

            sender.balance -= amount;
            receiver.balance += amount;

            db.update_user(&sender).map_err(|e| e.to_string())?;
            db.update_user(&receiver).map_err(|e| e.to_string())?;
        }

        Ok(())
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
