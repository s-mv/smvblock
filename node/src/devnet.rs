use crate::config::NodeConfig;
use crate::node::{Node, NodeError, NodeType};
use futures::future::{join_all, pending};
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
                let db_path = node.config.database_path();
                if db_path.exists() {
                    std::fs::remove_file(&db_path).ok();
                    println!("Deleted database file: {}", db_path.display());
                }
                node.p2p.db.init()?;
            }
        }

        let local_set = LocalSet::new();

        local_set
            .run_until(async {
                println!("Starting seed nodes...");
                for node in self.seed_nodes.clone() {
                    self.spawn_node_task(&local_set, node);
                }

                println!("Starting normal nodes...");
                for node in self.normal_nodes.clone() {
                    self.spawn_node_task(&local_set, node);
                }

                println!("Starting shallow nodes...");
                for node in self.shallow_nodes.clone() {
                    self.spawn_node_task(&local_set, node);
                }

                println!("Connecting nodes...");
                self.connect_nodes().await?;

                println!("All nodes are running");

                pending::<()>().await;

                #[allow(unreachable_code)]
                Ok(())
            })
            .await
    }

    fn spawn_node_task(&self, local_set: &LocalSet, node: Node) {
        local_set.spawn_local(async move {
            if let Err(e) = node.start().await {
                eprintln!("Node on {} failed: {}", node.listen_addr, e);
            }
        });
    }

    async fn connect_nodes(&self) -> Result<(), NodeError> {
        let mut receivers: Vec<_> = self
            .seed_nodes
            .iter()
            .map(|seed_node| seed_node.subscribe_ready())
            .collect();

        let futures = receivers.iter_mut().map(|rx| rx.recv());

        let _results = join_all(futures).await;

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
