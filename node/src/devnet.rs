use crate::config::NodeConfig;
use crate::node::{Node, NodeType, ReadyState};
use smv_core::Network;
use std::error::Error;
use std::net::SocketAddr;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use std::fs;
use std::path::Path;

pub struct DevnetConfig {
    pub seed_nodes: Vec<SocketAddr>,
    pub normal_nodes: Vec<SocketAddr>,
    pub shallow_nodes: Vec<SocketAddr>,
}

impl Default for DevnetConfig {
    fn default() -> Self {
        Self {
            seed_nodes: vec![
                "127.0.0.1:8000".parse().unwrap(),
                "127.0.0.1:8001".parse().unwrap(),
                "127.0.0.1:8002".parse().unwrap(),
                "127.0.0.1:8003".parse().unwrap(),
            ],
            normal_nodes: vec![
                "127.0.0.1:8010".parse().unwrap(),
                "127.0.0.1:8011".parse().unwrap(),
            ],
            shallow_nodes: vec!["127.0.0.1:8020".parse().unwrap()],
        }
    }
}

// this was hell
async fn start_node(config: NodeConfig) -> JoinHandle<Result<(), Box<dyn Error + Send + Sync>>> {
    let node = Node::new(config.clone());
    let mut rx = node.subscribe_ready();

    let node_handle = tokio::spawn(async move {
        node.start()
            .await
            .map_err(|e| e.to_string())
            .map_err(|e| e.into())
    });

    println!("Waiting for ready signal from {}", config.listen_addr);

    loop {
        match rx.recv().await {
            Ok(ReadyState::Ready) => {
                println!("Node on {} is ready", config.listen_addr);
                return node_handle;
            }
            Ok(ReadyState::Failed(e)) => {
                eprintln!("Node on {} failed to start: {}", config.listen_addr, e);
                node_handle.abort();
                return tokio::spawn(async { Err(e.into()) });
            }
            Ok(state) => {
                println!("Node on {} state: {:?}", config.listen_addr, state);
            }
            Err(broadcast::error::RecvError::Lagged(n)) => {
                println!(
                    "Receiver for {} lagged by {} messages, continuing...",
                    config.listen_addr, n
                );
            }
            Err(broadcast::error::RecvError::Closed) => {
                eprintln!(
                    "Ready channel closed unexpectedly for {}",
                    config.listen_addr
                );
                node_handle.abort();
                return tokio::spawn(async { Err("Ready channel closed".into()) });
            }
        }
    }
}

pub async fn start_devnet(reset_db: bool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = DevnetConfig::default();
    let db_path = Path::new("blockchain.db");

    if reset_db {
        fs::remove_file(db_path).unwrap_or_else(|_| ());
    }

    let mut handles = Vec::new();

    println!("Starting seed nodes...");
    for addr in config.seed_nodes.clone() {
        let node_config = NodeConfig::new(NodeType::Seed, Network::Devnet, Some(addr), None);
        let handle = start_node(node_config).await;
        handles.push(handle);
    }
    println!("All seed nodes are ready");

    println!("Starting normal nodes...");
    for addr in config.normal_nodes.clone() {
        let node_config = NodeConfig::new(
            NodeType::Normal,
            Network::Devnet,
            Some(addr),
            Some(config.seed_nodes[0]),
        );
        let handle = start_node(node_config).await;
        handles.push(handle);
    }

    println!("Starting shallow nodes...");
    for addr in config.shallow_nodes.clone() {
        let node_config = NodeConfig::new(
            NodeType::Shallow,
            Network::Devnet,
            Some(addr),
            Some(config.seed_nodes[0]),
        );
        let handle = start_node(node_config).await;
        handles.push(handle);
    }

    println!("All nodes are running");

    for handle in handles {
        match handle.await {
            Ok(Ok(())) => (),
            Ok(Err(e)) => eprintln!("Node exited with error: {}", e),
            Err(e) => eprintln!("Node task panicked: {}", e),
        }
    }

    Ok(())
}
