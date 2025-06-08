use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::sync::broadcast;

use crate::config::NodeConfig;
use crate::p2p::P2P;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeType {
    Seed,
    Normal,
    Shallow,
}

#[derive(Debug, Clone)]
pub enum ReadyState {
    Starting,
    Ready,
    Running,
    Failed(String),
}

#[derive(Clone)]
pub struct Node {
    node_type: NodeType,
    listen_addr: SocketAddr,
    seed_addr: Option<SocketAddr>,
    config: NodeConfig,
    p2p: P2P,
    ready_tx: broadcast::Sender<ReadyState>,
}

impl Node {
    pub fn new(config: NodeConfig) -> Self {
        let (ready_tx, _) = broadcast::channel(1);
        let p2p = P2P::new(
            config.node_type.clone(),
            config.network.clone(),
            config.listen_addr,
        );

        Self {
            node_type: config.node_type.clone(),
            listen_addr: config.listen_addr,
            seed_addr: config.seed_addr,
            p2p,
            config,
            ready_tx,
        }
    }

    pub fn subscribe_ready(&self) -> broadcast::Receiver<ReadyState> {
        self.ready_tx.subscribe()
    }

    pub async fn ready(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.ready_tx.send(ReadyState::Starting)?;
        self.init_database()?;
        self.p2p.init().await?;

        match &self.node_type {
            NodeType::Seed => {
                println!("Seed node ready on {}", self.listen_addr);
            }
            NodeType::Normal | NodeType::Shallow => {
                println!(
                    "{} node ready on {}, will connect to seed: {}",
                    format!("{:?}", self.node_type).to_lowercase(),
                    self.listen_addr,
                    self.seed_addr.expect("Seed address required")
                );
            }
        }

        self.ready_tx.send(ReadyState::Ready)?;
        Ok(())
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.ready_tx.send(ReadyState::Running)?;

        match &self.node_type {
            NodeType::Seed => {
                if let Err(e) = self.p2p.run().await {
                    self.ready_tx.send(ReadyState::Failed(e.to_string()))?;
                    return Err(e);
                }
            }
            NodeType::Normal | NodeType::Shallow => {
                if let Some(seed) = self.seed_addr {
                    if let Err(e) = self.p2p.connect_to_peer(seed).await {
                        self.ready_tx.send(ReadyState::Failed(e.to_string()))?;
                        return Err(e);
                    }
                }
                if let Err(e) = self.p2p.run().await {
                    self.ready_tx.send(ReadyState::Failed(e.to_string()))?;
                    return Err(e);
                }
            }
        }

        Ok(())
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.ready().await?;
        self.run().await
    }

    fn init_database(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut connection = Connection::open(self.config.database_path())?;

        connection.execute(
            "CREATE TABLE IF NOT EXISTS accounts (
                address TEXT PRIMARY KEY,
                balance INTEGER NOT NULL,
                nonce INTEGER NOT NULL,
                public_key TEXT
            )",
            [],
        )?;

        connection.execute(
            "CREATE TABLE IF NOT EXISTS blocks (
                hash TEXT PRIMARY KEY,
                previous_hash TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                nonce INTEGER NOT NULL
            )",
            [],
        )?;

        connection.execute(
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

        if matches!(self.config.network, smv_core::Network::Devnet) {
            let count: i64 =
                connection.query_row("SELECT COUNT(*) FROM accounts", [], |row| row.get(0))?;

            if count == 0 {
                // POKEMON
                let accounts: HashMap<&str, u64> = [
                    ("Pikachu", 1000),
                    ("Geodude", 800),
                    ("Snorlax", 650),
                    ("Pidgeot", 700),
                    ("Haunter", 900),
                ]
                .iter()
                .cloned()
                .collect();

                let tx = connection.transaction()?;
                for (name, balance) in accounts {
                    tx.execute(
                        "INSERT OR REPLACE INTO accounts (address, balance, nonce) VALUES (?1, ?2, 0)",
                        [name, &balance.to_string()],
                    )?;
                }
                tx.commit()?;
            }
        }

        Ok(())
    }
}
