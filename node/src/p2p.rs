use crate::node::NodeType;
use serde::{Deserialize, Serialize};
use smv_core::Network;
use smv_core::blockchain::Blockchain;
use std::collections::HashMap;
use std::error::Error;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};

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
        result: Result<String, String>,
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
    blockchain: Arc<Mutex<Blockchain>>, // shared blockchain instance
}

impl P2P {
    pub fn new(
        node_type: NodeType,
        network: Network,
        address: SocketAddr,
        db_path: &Path,
    ) -> Result<Self, Box<dyn Error>> {
        let blocks = Blockchain::load_blocks_from_db(db_path)?;
        let blockchain = Blockchain::from_blocks(blocks);

        Ok(Self {
            node_type,
            network,
            address,
            peers: Arc::new(Mutex::new(HashMap::new())),
            head_hash: Arc::new(Mutex::new(String::from("genesis"))),
            height: Arc::new(Mutex::new(0)),
            blockchain: Arc::new(Mutex::new(blockchain)),
        })
    }

    pub async fn init(&self) -> Result<(), Box<dyn Error>> {
        let _listener = TcpListener::bind(self.address).await?;
        Ok(())
    }

    pub async fn run(&self, db_path: &Path) -> Result<(), Box<dyn Error>> {
        let listener = TcpListener::bind(self.address).await?;
        let peers = self.peers.clone();
        let head_hash = self.head_hash.clone();
        let height = self.height.clone();
        let blockchain = self.blockchain.clone();

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
            let blockchain = blockchain.clone();
            let db_path = db_path.to_path_buf();

            let p2p = self.clone();

            tokio::spawn(async move {
                if let Err(e) = p2p
                    .handle_connection(
                        socket, peer_addr, peers, head_hash, height, blockchain, &db_path,
                    )
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

    pub async fn connect_to_peer(&self, addr: SocketAddr) -> Result<(), Box<dyn Error>> {
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
        &self,
        stream: TcpStream,
        peer_addr: SocketAddr,
        peers: Arc<Mutex<HashMap<SocketAddr, (NodeType, Instant)>>>,
        head_hash: Arc<Mutex<String>>,
        height: Arc<Mutex<u64>>,
        blockchain: Arc<Mutex<Blockchain>>,
        db_path: &Path,
    ) -> Result<(), Box<dyn Error>> {
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
                    if peer_network != self.network.as_str() {
                        eprintln!(
                            "[{}] Rejected peer {} - network mismatch",
                            self.network.as_str().to_uppercase(),
                            address
                        );
                        return Ok(());
                    }

                    if address != peer_addr {
                        eprintln!(
                            "[{}] Warning: Peer claims to be {}, but connected from {}",
                            self.network.as_str().to_uppercase(),
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
                Message::SendTransaction { to, amount } => {
                    let blockchain = blockchain.clone();
                    let response = {
                        let mut blockchain = blockchain.lock().await;

                        let sender_keypair = smv_core::crypto::generate_keypair();
                        let sender_address =
                            smv_core::crypto::public_key_to_address(&sender_keypair.verifying_key);

                        let receiver_address: smv_core::crypto::Address = match hex::decode(&to) {
                            Ok(decoded) => match decoded.try_into() {
                                Ok(addr) => addr,
                                Err(_) => {
                                    eprintln!("Invalid receiver address length: {}", to);
                                    return Ok(());
                                }
                            },
                            Err(_) => {
                                eprintln!("Invalid receiver address format: {}", to);
                                return Ok(());
                            }
                        };

                        let expected_nonce = blockchain.state.get_nonce(&sender_address);

                        let transaction = smv_core::transaction::Transaction::new(
                            &sender_keypair,
                            receiver_address,
                            amount,
                            expected_nonce,
                        );

                        match transaction.validate(
                            smv_core::transaction::ValidationLevel::Full,
                            Some(&blockchain.state),
                        ) {
                            Ok(_) => match blockchain.add_transaction(transaction.clone()) {
                                Ok(_) => Message::TransactionResponse {
                                    result: Ok(hex::encode(transaction.hash())),
                                },
                                Err(e) => {
                                    eprintln!("Failed to add transaction: {}", e);
                                    Message::TransactionResponse {
                                        result: Err("error".to_string()),
                                    }
                                }
                            },
                            Err(e) => {
                                eprintln!("Transaction validation failed: {}", e);
                                Message::TransactionResponse {
                                    result: Err("validation error".to_string()),
                                }
                            }
                        }
                    };

                    let blockchain = blockchain.lock().await;
                    blockchain.save_blocks_to_db(db_path)?;

                    blockchain.save_blocks_to_db(db_path)?;

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

    pub async fn get_peers(&self, addr: SocketAddr) -> Result<Vec<SocketAddr>, Box<dyn Error>> {
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

    pub async fn get_status(&self, addr: SocketAddr) -> Result<(String, u64), Box<dyn Error>> {
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
