use crate::config::NodeConfig;
use crate::node::{Node, NodeError, NodeType};
use smv_core::Network;
use tokio::task::{LocalSet, spawn_local};

pub struct Devnet {
    pub seed_nodes: Vec<Node>,
    pub normal_nodes: Vec<Node>,
    pub shallow_nodes: Vec<Node>,
}

impl Devnet {
    pub fn default() -> Self {
        Self {
            seed_nodes: vec![
                Node::new(NodeConfig::new(
                    NodeType::Seed,
                    Network::Devnet,
                    Some("127.0.0.1:8000".parse().unwrap()),
                    None,
                )),
                Node::new(NodeConfig::new(
                    NodeType::Seed,
                    Network::Devnet,
                    Some("127.0.0.1:8001".parse().unwrap()),
                    None,
                )),
                Node::new(NodeConfig::new(
                    NodeType::Seed,
                    Network::Devnet,
                    Some("127.0.0.1:8002".parse().unwrap()),
                    None,
                )),
            ],
            normal_nodes: vec![
                Node::new(NodeConfig::new(
                    NodeType::Normal,
                    Network::Devnet,
                    Some("127.0.0.1:8010".parse().unwrap()),
                    Some("127.0.0.1:8000".parse().unwrap()),
                )),
                Node::new(NodeConfig::new(
                    NodeType::Normal,
                    Network::Devnet,
                    Some("127.0.0.1:8011".parse().unwrap()),
                    Some("127.0.0.1:8001".parse().unwrap()),
                )),
            ],
            shallow_nodes: vec![Node::new(NodeConfig::new(
                NodeType::Shallow,
                Network::Devnet,
                Some("127.0.0.1:8020".parse().unwrap()),
                Some("127.0.0.1:8002".parse().unwrap()),
            ))],
        }
    }

    pub fn new(seed_nodes: Vec<Node>, normal_nodes: Vec<Node>, shallow_nodes: Vec<Node>) -> Self {
        Self {
            seed_nodes,
            normal_nodes,
            shallow_nodes,
        }
    }

    pub async fn start(&self, reset_db: bool) -> Result<(), NodeError> {
        if reset_db {
            println!("Resetting databases...");
            for node in self
                .seed_nodes
                .iter()
                .chain(self.normal_nodes.iter())
                .chain(self.shallow_nodes.iter())
            {
                let p2p = node.p2p.clone();
                p2p.db.delete_db()?;
                p2p.db.init()?;
                println!("Database reset for node at {}", node.listen_addr);
            }
        }

        let local_set = LocalSet::new();
        local_set
            .run_until(async {
                println!("Starting seed nodes...");
                for node in self.seed_nodes.clone() {
                    Self::start_node(node).await?;
                }

                println!("Starting normal nodes...");
                for node in self.normal_nodes.clone() {
                    Self::start_node(node).await?;
                }

                println!("Starting shallow nodes...");
                for node in self.shallow_nodes.clone() {
                    Self::start_node(node).await?;
                }

                println!("Connecting nodes...");
                self.connect_nodes().await?;

                println!("All nodes are running");
                Ok(())
            })
            .await
    }

    async fn start_node(node: Node) -> Result<(), NodeError> {
        println!("Starting node on {}", node.listen_addr);

        spawn_local(async move {
            if let Err(e) = node.start().await {
                eprintln!("Node on {} failed: {}", node.listen_addr, e);
            }
        });

        Ok(())
    }

    async fn connect_nodes(&self) -> Result<(), NodeError> {
        // Wait for seed nodes to be ready
        for seed_node in &self.seed_nodes {
            seed_node.subscribe_ready().recv().await.ok();
        }

        for normal_node in &self.normal_nodes {
            for seed_node in &self.seed_nodes {
                normal_node.connect_to_node(seed_node.listen_addr).await?;
            }
        }

        for shallow_node in &self.shallow_nodes {
            for seed_node in &self.seed_nodes {
                shallow_node.connect_to_node(seed_node.listen_addr).await?;
            }
        }

        Ok(())
    }
}
