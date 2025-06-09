use rusqlite::{Connection, Result};
use smv_core::block::Block;
use std::fs::remove_file;
use std::path::Path;

pub struct Database {
    pub connection: Connection,
    _not_send_sync: std::marker::PhantomData<*const ()>, // don't even ask
}

// this makes me stay up during the night
// this is the cost of learning rust
impl Clone for Database {
    fn clone(&self) -> Self {
        let conn = Connection::open(self.connection.path().unwrap()).unwrap();
        Self {
            connection: conn,
            _not_send_sync: std::marker::PhantomData,
        }
    }
}

impl Database {
    pub fn new(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        Ok(Self {
            connection: conn,
            _not_send_sync: std::marker::PhantomData,
        })
    }

    pub fn init(&self) -> Result<()> {
        self.connection.execute(
            "CREATE TABLE IF NOT EXISTS accounts (
                address TEXT PRIMARY KEY,
                balance INTEGER NOT NULL,
                nonce INTEGER NOT NULL
            )",
            [],
        )?;

        self.connection.execute(
            "CREATE TABLE IF NOT EXISTS blocks (
                hash TEXT PRIMARY KEY,
                previous_hash TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                nonce INTEGER NOT NULL,
                transactions TEXT NOT NULL
            )",
            [],
        )?;

        Ok(())
    }

    pub fn save_block(&self, block: &Block) -> Result<()> {
        let transactions = serde_json::to_string(&block.transactions).ok();
        self.connection.execute(
            "INSERT INTO blocks (hash, previous_hash, timestamp, nonce, transactions) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                hex::encode(block.hash),
                hex::encode(block.previous_hash),
                block.timestamp.timestamp(),
                block.nonce,
                transactions
            ],
        )?;
        Ok(())
    }

    pub fn save_blocks(&self, blocks: &[Block]) -> Result<()> {
        for block in blocks {
            self.save_block(block)?;
        }
        Ok(())
    }

    pub fn load_blocks(&self) -> Result<Vec<Block>> {
        let mut stmt = self
            .connection
            .prepare("SELECT * FROM blocks ORDER BY timestamp ASC")?;
        let rows = stmt.query_map([], |row| {
            let hash: String = row.get(0)?;
            let previous_hash: String = row.get(1)?;
            let timestamp: i64 = row.get(2)?;
            let nonce: u64 = row.get(3)?;
            let transactions: String = row.get(4)?;

            let block = Block {
                hash: hex::decode(hash).unwrap().try_into().unwrap(),
                previous_hash: hex::decode(previous_hash).unwrap().try_into().unwrap(),
                timestamp: chrono::DateTime::from_timestamp(timestamp, 0)
                    .unwrap()
                    .into(),
                nonce,
                transactions: serde_json::from_str(&transactions).unwrap(),
            };
            Ok(block)
        })?;

        rows.collect()
    }

    pub fn delete_db(&self) -> Result<()> {
        let db_path = self.connection.path().unwrap();
        if Path::new(db_path).exists() {
            remove_file(db_path).ok();
            println!("Deleted database file: {}", Path::new(db_path).display());
        }
        Ok(())
    }
}
