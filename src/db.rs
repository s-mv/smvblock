use rusqlite::{Connection, OptionalExtension, Result};
use std::path::PathBuf;

use crate::blockchain::{Block, Transaction, User};

pub struct Database {
    path: PathBuf,
    conn: Connection,
    test: bool,
}

impl Database {
    pub fn new(path: Option<&str>, test: bool) -> Result<Self> {
        let db_path = if test {
            let test_path = dirs::home_dir().unwrap().join(".smvblock/test.db");
            if test_path.exists() {
                std::fs::remove_file(&test_path).unwrap();
            }
            test_path
        } else {
            path.map(PathBuf::from)
                .unwrap_or_else(|| dirs::home_dir().unwrap().join(".smvblock/temp.db"))
        };

        let conn = Connection::open(&db_path)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                address BLOB NOT NULL,
                public_key BLOB NOT NULL,
                balance INTEGER NOT NULL,
                stake INTEGER NOT NULL)",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS blocks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                previous_hash BLOB NOT NULL,
                merkle_root BLOB NOT NULL,
                nonce INTEGER NOT NULL,
                timestamp INTEGER NOT NULL)",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS transactions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                block_id INTEGER,
                sender BLOB NOT NULL,
                receiver BLOB NOT NULL,
                amount INTEGER NOT NULL,
                nonce INTEGER NOT NULL,
                signature BLOB NOT NULL,
                sender_public_key BLOB NOT NULL,
                FOREIGN KEY (block_id) REFERENCES blocks(id))",
            [],
        )?;

        Ok(Database {
            path: db_path,
            conn,
            test,
        })
    }

    pub fn add_block(&mut self, block: &Block) -> Result<()> {
        let transaction = self.conn.transaction().unwrap();

        transaction.execute(
            "INSERT INTO blocks (previous_hash, merkle_root, nonce, timestamp) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                block.previous_hash,
                block.merkle_root,
                block.nonce,
                block.timestamp,
            ],
        )?;

        let block_id = transaction.last_insert_rowid();

        for tx in &block.transactions {
            transaction.execute(
                "INSERT INTO transactions (block_id, sender, receiver, amount, nonce, signature, sender_public_key)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    block_id,
                    tx.sender,
                    tx.receiver,
                    tx.amount,
                    tx.nonce,
                    tx.signature,
                    tx.sender_public_key
                ],
            )?;
        }

        transaction.commit()?;
        Ok(())
    }

    pub fn get_block(&self, hash: &[u8]) -> Result<Option<Block>> {
        let mut stmt = self.conn.prepare(
            "SELECT previous_hash, merkle_root, nonce, timestamp FROM blocks WHERE previous_hash = ?1",
        )?;

        let block = stmt
            .query_row(rusqlite::params![hash], |row| {
                Ok(Block {
                    previous_hash: row.get(0)?,
                    merkle_root: row.get(1)?,
                    nonce: row.get(2)?,
                    timestamp: row.get(3)?,
                    transactions: vec![],
                })
            })
            .optional()?;

        Ok(block)
    }

    pub fn get_blocks(&self) -> Result<Vec<Block>> {
        let mut stmt = self
            .conn
            .prepare("SELECT previous_hash, merkle_root, nonce, timestamp FROM blocks")?;

        let blocks = stmt
            .query_map([], |row| {
                Ok(Block {
                    previous_hash: row.get(0)?,
                    merkle_root: row.get(1)?,
                    nonce: row.get(2)?,
                    timestamp: row.get(3)?,
                    transactions: vec![],
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(blocks)
    }

    pub fn add_transaction(&self, transaction: &Transaction) -> Result<()> {
        self.conn.execute(
            "INSERT INTO transactions (sender, receiver, amount, nonce, signature, sender_public_key) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![transaction.sender, transaction.receiver, transaction.amount, transaction.nonce, transaction.signature, transaction.sender_public_key],
        )?;
        Ok(())
    }

    pub fn get_transactions(&self) -> Result<Vec<Transaction>> {
        let mut stmt = self.conn.prepare("SELECT sender, receiver, amount, nonce, signature, sender_public_key FROM transactions")?;
        let transactions = stmt
            .query_map([], |row| {
                Ok(Transaction {
                    sender: row.get(0)?,
                    receiver: row.get(1)?,
                    amount: row.get(2)?,
                    nonce: row.get(3)?,
                    signature: row.get(4)?,
                    sender_public_key: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(transactions)
    }

    pub fn add_user(&self, user: &User) -> Result<()> {
        self.conn.execute(
            "INSERT INTO users (address, public_key, balance, stake) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![user.address, user.public_key, user.balance, user.stake],
        )?;
        Ok(())
    }

    pub fn get_users(&self) -> Result<Vec<User>> {
        let mut stmt = self
            .conn
            .prepare("SELECT address, public_key, balance, stake FROM users")?;
        let users = stmt
            .query_map([], |row| {
                Ok(User {
                    address: row.get(0)?,
                    public_key: row.get(1)?,
                    balance: row.get(2)?,
                    stake: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(users)
    }

    pub fn get_user(&self, address: &[u8]) -> Result<Option<User>> {
        let mut stmt = self
            .conn
            .prepare("SELECT address, public_key, balance, stake FROM users WHERE address = ?1")?;
        let user = stmt
            .query_row(rusqlite::params![address], |row| {
                Ok(User {
                    address: row.get(0)?,
                    public_key: row.get(1)?,
                    balance: row.get(2)?,
                    stake: row.get(3)?,
                })
            })
            .optional()?;
        Ok(user)
    }

    pub fn get_nonce(&self, address: &[u8]) -> Result<u64> {
        let mut stmt = self
            .conn
            .prepare("SELECT COUNT(*) FROM transactions WHERE sender = ?1")?;
        let nonce: u64 = stmt.query_row(rusqlite::params![address], |row| row.get(0))?;
        Ok(nonce)
    }

    pub fn update_user(&self, user: &User) -> Result<()> {
        self.conn.execute(
            "UPDATE users SET balance = ?1, stake = ?2 WHERE address = ?3",
            rusqlite::params![user.balance, user.stake, user.address],
        )?;
        Ok(())
    }

    pub fn delete_user(&self, address: &[u8]) -> Result<()> {
        self.conn.execute(
            "DELETE FROM users WHERE address = ?1",
            rusqlite::params![address],
        )?;
        Ok(())
    }
}
