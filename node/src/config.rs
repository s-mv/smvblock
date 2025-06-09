use serde::{Deserialize, Serialize};
use smv_core::Network;
use std::net::SocketAddr;
use std::path::PathBuf;

use crate::node::NodeType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub node_type: NodeType,
    pub listen_addr: SocketAddr,
    pub seed_addr: Option<SocketAddr>,
    pub network: Network,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            node_type: NodeType::Shallow,
            listen_addr: "127.0.0.1:0".parse().unwrap(),
            seed_addr: None,
            network: Network::Devnet,
        }
    }
}

impl NodeConfig {
    pub fn new(
        node_type: NodeType,
        network: Network,
        listen_addr: Option<SocketAddr>,
        connect_to: Option<SocketAddr>,
    ) -> Self {
        let listen_addr = match (node_type.clone(), listen_addr) {
            (NodeType::Seed, None) => default_seed_nodes(&network)[0],
            (_, None) => "127.0.0.1:0".parse().unwrap(),
            (_, Some(addr)) => addr,
        };

        Self {
            node_type,
            network,
            listen_addr,
            seed_addr: connect_to,
        }
    }

    pub fn database_path(&self) -> PathBuf {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let formatted_addr = self.listen_addr.to_string().replace(":", "_");
        let path = home_dir
            .join(".smvblock")
            .join(format!("{}.db", formatted_addr));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        path
    }
}

pub fn default_seed_nodes(network: &Network) -> Vec<SocketAddr> {
    match network {
        Network::Devnet => vec![
            "127.0.0.1:8001".parse().unwrap(),
            "127.0.0.1:8002".parse().unwrap(),
            "127.0.0.1:8003".parse().unwrap(),
        ],
        // TODO repl.it, fly.io, or other public seed nodes
        Network::Mainnet => vec![
            "127.0.0.1:4001".parse().unwrap(),
            "127.0.0.1:4002".parse().unwrap(),
            "127.0.0.1:4003".parse().unwrap(),
        ],
    }
}
