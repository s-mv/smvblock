use crate::config::NodeConfig;
use crate::node::{Node, NodeType, ReadyState};
use smv_core::Network;
use std::error::Error;
use std::net::SocketAddr;

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

async fn start_node(config: NodeConfig) -> Result<(), Box<dyn Error + Send + Sync>> {
    let node = Node::new(config.clone());
    let mut rx = node.subscribe_ready();

    let node_handle = tokio::spawn(async move { node.start().await.map_err(|e| e.to_string()) });

    println!("Waiting for ready signal from {}", config.listen_addr);
    let ready_result = rx.recv().await;
    println!(
        "Received result from {}: {:?}",
        config.listen_addr, ready_result
    );

    match ready_result {
        Ok(ReadyState::Ready) => {
            println!("Node on {} is ready", config.listen_addr);
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            if node_handle.is_finished() {
                match node_handle.await {
                    Ok(Ok(())) => Ok(()),
                    Ok(Err(e)) => Err(e.into()),
                    Err(e) => Err(format!("Node task panicked: {}", e).into()),
                }
            } else {
                Ok(())
            }
        }
        Ok(ReadyState::Failed(e)) => Err(format!("Node failed: {}", e).into()),
        Err(recv_error) => {
            println!(
                "Channel recv error for {}: {:?}",
                config.listen_addr, recv_error
            );
            Err(format!("Node startup timed out: {:?}", recv_error).into())
        }
        other => {
            println!(
                "Unexpected ready state for {}: {:?}",
                config.listen_addr, other
            );
            Err(format!("Unexpected node state: {:?}", other).into())
        }
    }
}
pub async fn start_devnet() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = DevnetConfig::default();

    println!("Starting seed nodes...");
    for addr in config.seed_nodes.clone() {
        let node_config = NodeConfig::new(NodeType::Seed, Network::Devnet, Some(addr), None);
        if let Err(e) = start_node(node_config).await {
            eprintln!("Seed node {} failed to start: {}", addr, e);
            return Err(e);
        }
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
        if let Err(e) = start_node(node_config).await {
            eprintln!("Normal node {} failed to start: {}", addr, e);
            return Err(e);
        }
    }

    println!("Starting shallow nodes...");
    for addr in config.shallow_nodes.clone() {
        let node_config = NodeConfig::new(
            NodeType::Shallow,
            Network::Devnet,
            Some(addr),
            Some(config.seed_nodes[0]),
        );
        if let Err(e) = start_node(node_config).await {
            eprintln!("Shallow node {} failed to start: {}", addr, e);
            return Err(e);
        }
    }

    println!("All nodes are running");
    Ok(())
}
