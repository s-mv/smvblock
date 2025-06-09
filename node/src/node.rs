use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use thiserror::Error;
use tokio::sync::broadcast;

use crate::config::NodeConfig;
use crate::p2p::P2P;

#[derive(Error, Debug)]
pub enum NodeError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] rusqlite::Error),
    #[error("P2P error: {0}")]
    P2PError(String),
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Address parsing error: {0}")]
    AddrParseError(#[from] std::net::AddrParseError),
    #[error("Other error: {0}")]
    Other(String),
}

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
    _Failed(String), // TODO use this too
}

#[derive(Clone)]
pub struct Node {
    pub node_type: NodeType,
    pub listen_addr: SocketAddr,
    pub seed_addr: Option<SocketAddr>,
    pub config: NodeConfig,
    pub p2p: P2P,
    pub ready_tx: broadcast::Sender<ReadyState>,
}

impl Node {
    pub fn new(config: NodeConfig) -> Self {
        let (ready_tx, _) = broadcast::channel(16);
        let p2p = P2P::new(
            config.node_type.clone(),
            config.network.clone(),
            config.listen_addr,
            config.database_path().as_path(),
        );

        Self {
            node_type: config.node_type.clone(),
            listen_addr: config.listen_addr,
            seed_addr: config.seed_addr,
            config,
            p2p: p2p.unwrap(),
            ready_tx,
        }
    }

    pub fn subscribe_ready(&self) -> broadcast::Receiver<ReadyState> {
        self.ready_tx.subscribe()
    }

    pub async fn ready(&self) -> Result<(), NodeError> {
        self.ready_tx.send(ReadyState::Starting).ok();
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

        self.ready_tx.send(ReadyState::Ready).ok();
        Ok(())
    }

    pub async fn run(&self) -> Result<(), NodeError> {
        self.ready_tx.send(ReadyState::Running).ok();

        match &self.node_type {
            NodeType::Seed => {
                self.p2p.run(self.config.database_path().as_path()).await?;
            }
            NodeType::Normal | NodeType::Shallow => {
                if let Some(seed) = self.seed_addr {
                    self.p2p.connect_to_peer(seed).await?;
                }
                self.p2p.run(self.config.database_path().as_path()).await?;
            }
        }

        Ok(())
    }

    pub async fn start(&self) -> Result<(), NodeError> {
        self.ready().await?;
        self.run().await
    }

    pub(crate) async fn connect_to_node(&self, listen_addr: SocketAddr) -> Result<(), NodeError> {
        println!(
            "Node at {} attempting to connect to node at {}...",
            self.listen_addr, listen_addr
        );

        match self.p2p.connect_to_peer(listen_addr).await {
            Ok(_) => {
                println!(
                    "Node at {} successfully connected to node at {}",
                    self.listen_addr, listen_addr
                );
                Ok(())
            }
            Err(e) => {
                eprintln!(
                    "Node at {} failed to connect to node at {}: {}",
                    self.listen_addr, listen_addr, e
                );
                Err(NodeError::P2PError(format!(
                    "Failed to connect to node at {}: {}",
                    listen_addr, e
                )))
            }
        }
    }
}
