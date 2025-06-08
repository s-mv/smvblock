mod config;
mod devnet;
mod node;
mod p2p;
mod storage;

use clap::{Parser, ValueEnum};
use config::NodeConfig;
use node::{Node, NodeType};
use smv_core::Network;
use std::net::SocketAddr;

#[derive(Debug, Clone, ValueEnum)]
enum Mode {
    Seed,
    Normal,
    Shallow,
}

impl From<Mode> for NodeType {
    fn from(mode: Mode) -> Self {
        match mode {
            Mode::Seed => NodeType::Seed,
            Mode::Normal => NodeType::Normal,
            Mode::Shallow => NodeType::Shallow,
        }
    }
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(long, value_enum, default_value_t = Mode::Shallow)]
    mode: Mode,

    #[arg(long)]
    listen_addr: Option<SocketAddr>,

    #[arg(long)]
    connect_to: Option<SocketAddr>,

    #[arg(long, default_value = "devnet")]
    network: String,

    #[arg(long)]
    devnet: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if cli.devnet {
        devnet::start_devnet().await.unwrap();
    } else {
        let network = match cli.network.as_str() {
            "devnet" => Network::Devnet,
            "mainnet" => Network::Mainnet,
            _ => {
                eprintln!("Invalid network: {}", cli.network);
                std::process::exit(1);
            }
        };

        let node_type = NodeType::from(cli.mode);
        println!(
            "[{}] Starting {} node...",
            network.as_str().to_uppercase(),
            format!("{:?}", node_type).to_lowercase()
        );

        match node_type {
            NodeType::Seed if cli.connect_to.is_some() => {
                eprintln!("Seed nodes should not connect to other seeds");
                std::process::exit(1);
            }
            NodeType::Normal | NodeType::Shallow if cli.connect_to.is_none() => {
                eprintln!("--connect-to is required for normal and shallow modes");
                std::process::exit(1);
            }
            _ => {}
        }

        let config = NodeConfig::new(node_type, network, cli.listen_addr, cli.connect_to);
        let node = Node::new(config);
        node.start().await?;
    }

    Ok(())
}
