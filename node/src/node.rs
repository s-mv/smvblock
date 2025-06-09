use serde::{Deserialize, Serialize};
use smv_core::interface::handshake;
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NodeType {
    Seed,
    Normal,
    Shallow,
}

impl std::fmt::Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl NodeType {
    pub fn evaluate(string: String) -> NodeType {
        match string.to_lowercase().as_str() {
            "seed" => NodeType::Seed,
            "normal" => NodeType::Normal,
            "shallow" => NodeType::Shallow,
            _ => panic!("Unknown NodeType: {}", string), // TODO, more elegant handling
        }
    }
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
                    format!("{:?}", self.node_type),
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
                self.p2p.run().await?;
            }
            NodeType::Normal | NodeType::Shallow => {
                if let Some(seed) = self.seed_addr {
                    self.p2p.connect_to_peer(seed).await?;
                }
                self.p2p.run().await?;
            }
        }

        Ok(())
    }

    pub async fn start(&self) -> Result<(), NodeError> {
        self.ready().await?;
        self.run().await
    }

    pub async fn connect_to_node(&self, listen_addr: SocketAddr) -> Result<(), NodeError> {
        handshake(
            listen_addr,
            self.listen_addr,
            self.node_type.to_string(),
            self.config.network.to_string(),
        )
        .await
        .map_err(|e| NodeError::Other(format!("Handshake failed: {}", e)))?;

        self.p2p.add_peer(listen_addr, NodeType::Normal).await;

        println!(
            "Node at {} successfully connected to node at {}",
            self.listen_addr, listen_addr
        );

        Ok(())
    }

    pub async fn add_peer(&self, addr: SocketAddr, node_type: NodeType) {
        self.p2p.add_peer(addr, node_type).await;
    }

    // TODO do we need this?
    // pub async fn remove_peer(&self, addr: SocketAddr) {
    //     self.p2p.remove_peer(addr).await;
    // }

    pub async fn list_peers(&self) -> Vec<SocketAddr> {
        self.p2p.list_peers().await
    }
}
