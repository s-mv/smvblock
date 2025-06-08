use crate::crypto::{Address, Hash};
use rusqlite::{Connection, Result};

pub fn init_database(path: &str) -> Result<Connection> {
    let conn = Connection::open(path)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS accounts (
            address TEXT PRIMARY KEY,
            balance INTEGER NOT NULL DEFAULT 0,
            nonce INTEGER NOT NULL DEFAULT 0,
            public_key TEXT
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS blocks (
            hash TEXT PRIMARY KEY,
            previous_hash TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            nonce INTEGER NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS transactions (
            hash TEXT PRIMARY KEY,
            sender TEXT NOT NULL,
            receiver TEXT NOT NULL,
            amount INTEGER NOT NULL,
            nonce INTEGER NOT NULL,
            signature TEXT NOT NULL,
            sender_public_key TEXT NOT NULL,
            block_hash TEXT,
            FOREIGN KEY(block_hash) REFERENCES blocks(hash)
        )",
        [],
    )?;

    Ok(conn)
}

pub fn seed_devnet_accounts(conn: &Connection) -> Result<()> {
    let accounts = [("PIKACHU", 1000000), ("GEODUDE", 500000)];

    let tx = conn.transaction()?;
    for (name, balance) in accounts.iter() {
        tx.execute(
            "INSERT OR IGNORE INTO accounts (address, balance, nonce) VALUES (?1, ?2, 0)",
            [name, &balance.to_string()],
        )?;
    }
    tx.commit()?;

    Ok(())
}

pub struct DbState {
    conn: Connection,
}

impl DbState {
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }

    pub fn get_balance(&self, address: &Address) -> Result<u64> {
        let address_str = hex::encode(address);
        self.conn
            .query_row(
                "SELECT balance FROM accounts WHERE address = ?1",
                [&address_str],
                |row| row.get(0),
            )
            .unwrap_or(Ok(0))
    }

    pub fn get_nonce(&self, address: &Address) -> Result<u64> {
        let address_str = hex::encode(address);
        self.conn
            .query_row(
                "SELECT nonce FROM accounts WHERE address = ?1",
                [&address_str],
                |row| row.get(0),
            )
            .unwrap_or(Ok(0))
    }

    pub fn apply_transaction(
        &self,
        sender: &Address,
        receiver: &Address,
        amount: u64,
        nonce: u64,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;

        let sender_str = hex::encode(sender);
        let receiver_str = hex::encode(receiver);

        tx.execute(
            "UPDATE accounts SET balance = balance - ?1, nonce = ?2 WHERE address = ?3",
            [&amount.to_string(), &nonce.to_string(), &sender_str],
        )?;

        tx.execute(
            "INSERT INTO accounts (address, balance) VALUES (?1, ?2)
             ON CONFLICT(address) DO UPDATE SET balance = balance + ?2",
            [&receiver_str, &amount.to_string()],
        )?;

        tx.commit()?;
        Ok(())
    }
}
