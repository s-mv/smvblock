use crate::blockchain::{Block, Transaction, Transfer, User};
use rusqlite::{Connection, OptionalExtension, Result};
use std::path::PathBuf;

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
                let _ = std::fs::rename(&test_path, &test_path.with_extension("bak"));
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
                stake INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS blocks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                previous_hash BLOB NOT NULL,
                merkle_root BLOB NOT NULL,
                nonce INTEGER NOT NULL,
                timestamp INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS transactions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                tx_hash BLOB NOT NULL,
                receiver BLOB NOT NULL,
                amount INTEGER NOT NULL,
                nonce INTEGER NOT NULL,
                sender_public_key BLOB NOT NULL,
                signature BLOB NOT NULL,
                verified BOOLEAN NOT NULL
            )",
            [],
        )?;

        Ok(Database {
            path: db_path,
            conn,
            test,
        })
    }

    pub fn add_block(&mut self, block: &Block) -> Result<()> {
        let transaction = self.conn.transaction()?;

        transaction.execute(
            "INSERT INTO blocks (previous_hash, merkle_root, nonce, timestamp) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                block.previous_hash,
                block.merkle_root,
                block.nonce,
                block.timestamp,
            ],
        )?;

        for tx in &block.transactions {
            let tx_hash = tx.payload.hash();
            transaction.execute(
                "INSERT INTO transactions (tx_hash, receiver, amount, nonce, sender_public_key, signature, verified)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    tx_hash,
                    tx.payload.receiver,
                    tx.payload.amount,
                    tx.payload.nonce,
                    tx.sender_public_key,
                    tx.signature,
                    true,
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

    pub fn add_unsigned_transaction(&self, tx: &Transaction) -> Result<()> {
        self.add_transaction(tx, false)
    }

    pub fn add_signed_transaction(&self, tx: &Transaction) -> Result<()> {
        self.add_transaction(tx, true)
    }

    pub fn add_transaction(&self, transaction: &Transaction, verified: bool) -> Result<()> {
        let tx_hash = transaction.payload.hash();
        self.conn.execute(
            "INSERT INTO transactions (tx_hash, receiver, amount, nonce, sender_public_key, signature, verified) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                tx_hash,
                transaction.payload.receiver,
                transaction.payload.amount,
                transaction.payload.nonce,
                transaction.sender_public_key,
                transaction.signature,
                verified,
            ],
        )?;
        Ok(())
    }

    pub fn get_unverified_transactions(&self) -> Result<Vec<Transaction>> {
        self.get_transactions(false)
    }

    pub fn get_verified_transactions(&self) -> Result<Vec<Transaction>> {
        self.get_transactions(true)
    }

    pub fn get_all_transactions(&self) -> Result<Vec<Transaction>> {
        let query =
            "SELECT receiver, amount, nonce, sender_public_key, signature FROM transactions";

        let mut stmt = self.conn.prepare(&query)?;
        let transactions = stmt
            .query_map([], |row| {
                Ok(Transaction {
                    sender_public_key: row.get(3)?,
                    signature: row.get(4)?,
                    payload: Transfer {
                        receiver: row.get(0)?,
                        amount: row.get(1)?,
                        nonce: row.get(2)?,
                    },
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(transactions)
    }

    fn get_transactions(&self, verified: bool) -> Result<Vec<Transaction>> {
        let query = format!(
            "SELECT receiver, amount, nonce, sender_public_key, signature FROM transactions WHERE verified = {}",
            verified
        );

        let mut stmt = self.conn.prepare(&query)?;
        let transactions = stmt
            .query_map([], |row| {
                Ok(Transaction {
                    sender_public_key: row.get(3)?,
                    signature: row.get(4)?,
                    payload: Transfer {
                        receiver: row.get(0)?,
                        amount: row.get(1)?,
                        nonce: row.get(2)?,
                    },
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(transactions)
    }

    pub fn get_transaction_by_hash(&self, tx_hash: &[u8]) -> Result<Option<Transaction>> {
        let mut stmt = self.conn.prepare(
            "SELECT receiver, amount, nonce, sender_public_key, signature FROM transactions WHERE tx_hash = ?1",
        )?;

        let transaction = stmt
            .query_row(rusqlite::params![tx_hash], |row| {
                Ok(Transaction {
                    sender_public_key: row.get(3)?,
                    signature: row.get(4)?,
                    payload: Transfer {
                        receiver: row.get(0)?,
                        amount: row.get(1)?,
                        nonce: row.get(2)?,
                    },
                })
            })
            .optional()?;

        Ok(transaction)
    }

    pub fn update_transaction_verified(&self, tx_hash: &[u8], verified: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE transactions SET verified = ?1 WHERE tx_hash = ?2",
            rusqlite::params![verified, tx_hash],
        )?;
        Ok(())
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
            .prepare("SELECT COUNT(*) FROM transactions WHERE sender_public_key = ?1")?;

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

    pub fn get_total_stake(&self) -> Result<u64> {
        let mut stmt = self.conn.prepare("SELECT SUM(stake) FROM users")?;
        let total_stake: u64 = stmt.query_row([], |row| row.get(0))?;
        Ok(total_stake)
    }

    pub fn close(self) -> Result<(), rusqlite::Error> {
        match self.conn.close() {
            Ok(_) => Ok(()),
            Err((_, e)) => Err(e),
        }
    }
}
