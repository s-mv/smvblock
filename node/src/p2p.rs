use crate::node::NodeType;
use serde::{Deserialize, Serialize};
use smv_core::Network;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Message {
    Hello {
        address: SocketAddr,
        node_type: NodeType,
        network: String,
    },
    GetStatus,
    Status {
        head_hash: String,
        height: u64,
    },
    GetPeers,
    Peers(Vec<SocketAddr>),
    SendTransaction {
        to: String,
        amount: u64,
    },
    TransactionResponse {
        hash: String,
    },
}

const PEER_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Clone)]
pub struct P2P {
    node_type: NodeType,
    network: Network,
    address: SocketAddr,
    peers: Arc<Mutex<HashMap<SocketAddr, (NodeType, Instant)>>>,
    head_hash: Arc<Mutex<String>>,
    height: Arc<Mutex<u64>>,
}

impl P2P {
    pub fn new(node_type: NodeType, network: Network, address: SocketAddr) -> Self {
        Self {
            node_type,
            network,
            address,
            peers: Arc::new(Mutex::new(HashMap::new())),
            head_hash: Arc::new(Mutex::new(String::from("genesis"))),
            height: Arc::new(Mutex::new(0)),
        }
    }

    pub async fn init(&self) -> Result<(), Box<dyn std::error::Error>> {
        let _listener = TcpListener::bind(self.address).await?;
        Ok(())
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(self.address).await?;
        let peers = self.peers.clone();
        let head_hash = self.head_hash.clone();
        let height = self.height.clone();

        let cleanup_peers = peers.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(60)).await;
                let mut peers = cleanup_peers.lock().await;
                peers.retain(|_, (_, last_seen)| last_seen.elapsed() < PEER_TIMEOUT);
            }
        });

        loop {
            let (socket, peer_addr) = listener.accept().await?;
            let peers = peers.clone();
            let head_hash = head_hash.clone();
            let height = height.clone();
            let network = self.network.clone();
            tokio::spawn(async move {
                if let Err(e) =
                    Self::handle_connection(socket, peer_addr, peers, head_hash, height, network)
                        .await
                {
                    eprintln!("Error handling connection from {}: {}", peer_addr, e);
                }
            });
        }
    }

    pub fn get_address(&self) -> SocketAddr {
        self.address
    }

    pub async fn connect_to_peer(
        &self,
        addr: SocketAddr,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let stream = TcpStream::connect(addr).await?;
        let hello = Message::Hello {
            address: self.address,
            node_type: self.node_type.clone(),
            network: self.network.to_string(),
        };
        let msg = serde_json::to_string(&hello)?;
        let mut stream = tokio::io::BufWriter::new(stream);
        stream.write_all(msg.as_bytes()).await?;
        stream.write_all(b"\n").await?;
        Ok(())
    }

    async fn handle_connection(
        stream: TcpStream,
        peer_addr: SocketAddr,
        peers: Arc<Mutex<HashMap<SocketAddr, (NodeType, Instant)>>>,
        head_hash: Arc<Mutex<String>>,
        height: Arc<Mutex<u64>>,
        network: Network,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (read_half, write_half) = stream.into_split();
        let mut reader = tokio::io::BufReader::new(read_half);
        let mut writer = tokio::io::BufWriter::new(write_half);
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = reader.read_line(&mut line).await?;
            if bytes_read == 0 {
                break;
            }

            let msg: Message = serde_json::from_str(&line)?;
            match msg {
                Message::Hello {
                    address,
                    node_type,
                    network: peer_network,
                } => {
                    if peer_network != network.as_str() {
                        eprintln!(
                            "[{}] Rejected peer {} - network mismatch",
                            network.as_str().to_uppercase(),
                            address
                        );
                        return Ok(());
                    }

                    if address != peer_addr {
                        eprintln!(
                            "[{}] Warning: Peer claims to be {}, but connected from {}",
                            network.as_str().to_uppercase(),
                            address,
                            peer_addr
                        );
                    }

                    let mut peers = peers.lock().await;
                    peers.insert(peer_addr, (node_type, Instant::now()));
                }
                Message::GetStatus => {
                    let status = Message::Status {
                        head_hash: head_hash.lock().await.clone(),
                        height: *height.lock().await,
                    };
                    let response = serde_json::to_string(&status)?;
                    writer.write_all(response.as_bytes()).await?;
                    writer.write_all(b"\n").await?;
                    writer.flush().await?;
                }
                Message::GetPeers => {
                    let peers_list = {
                        let peers = peers.lock().await;
                        peers.keys().cloned().collect::<Vec<_>>()
                    };
                    let response = serde_json::to_string(&Message::Peers(peers_list))?;
                    writer.write_all(response.as_bytes()).await?;
                    writer.write_all(b"\n").await?;
                    writer.flush().await?;
                }
                Message::SendTransaction { to: _, amount: _ } => {
                    let response = Message::TransactionResponse {
                        hash: "dummy_hash".to_string(),
                    };
                    let response = serde_json::to_string(&response)?;
                    writer.write_all(response.as_bytes()).await?;
                    writer.write_all(b"\n").await?;
                    writer.flush().await?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    pub async fn get_peers(
        &self,
        addr: SocketAddr,
    ) -> Result<Vec<SocketAddr>, Box<dyn std::error::Error>> {
        let stream = TcpStream::connect(addr).await?;
        let (read_half, write_half) = stream.into_split();
        let mut reader = tokio::io::BufReader::new(read_half);
        let mut writer = tokio::io::BufWriter::new(write_half);

        let msg = serde_json::to_string(&Message::GetPeers)?;
        writer.write_all(msg.as_bytes()).await?;
        writer.write_all(b"\n").await?;

        let mut line = String::new();
        reader.read_line(&mut line).await?;

        if let Message::Peers(peers) = serde_json::from_str(&line)? {
            Ok(peers)
        } else {
            Err("Invalid response".into())
        }
    }

    pub async fn get_status(
        &self,
        addr: SocketAddr,
    ) -> Result<(String, u64), Box<dyn std::error::Error>> {
        let stream = TcpStream::connect(addr).await?;
        let (read_half, write_half) = stream.into_split();
        let mut reader = tokio::io::BufReader::new(read_half);
        let mut writer = tokio::io::BufWriter::new(write_half);

        let msg = serde_json::to_string(&Message::GetStatus)?;
        writer.write_all(msg.as_bytes()).await?;
        writer.write_all(b"\n").await?;

        let mut line = String::new();
        reader.read_line(&mut line).await?;

        if let Message::Status { head_hash, height } = serde_json::from_str(&line)? {
            Ok((head_hash, height))
        } else {
            Err("Invalid response".into())
        }
    }
}
