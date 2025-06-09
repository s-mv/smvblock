use crate::db::Database;
use crate::node::{NodeError, NodeType};
use serde::{Deserialize, Serialize};
use smv_core::Network;
use smv_core::blockchain::Blockchain;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::task::spawn_local;
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
    pub db: Database,
}

impl P2P {
    pub fn new(
        node_type: NodeType,
        network: Network,
        address: SocketAddr,
        db_path: &Path,
    ) -> Result<Self, NodeError> {
        let db = Database::new(db_path)?;
        db.init()?;
        let blocks = db.load_blocks()?;
        let blockchain = Blockchain::from_blocks(blocks);

        Ok(Self {
            node_type,
            network,
            address,
            peers: Arc::new(Mutex::new(HashMap::new())),
            head_hash: Arc::new(Mutex::new(String::from("genesis"))),
            height: Arc::new(Mutex::new(0)),
            blockchain: Arc::new(Mutex::new(blockchain)),
            db,
        })
    }

    pub async fn init(&self) -> Result<(), NodeError> {
        let _listener = TcpListener::bind(self.address).await?;
        Ok(())
    }

    pub async fn run(&self, db_path: &Path) -> Result<(), NodeError> {
        let listener = TcpListener::bind(self.address).await?;
        let peers = self.peers.clone();
        let head_hash = self.head_hash.clone();
        let height = self.height.clone();
        let blockchain = self.blockchain.clone();

        let cleanup_peers = peers.clone();
        spawn_local(async move {
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

            spawn_local(async move {
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

    pub async fn connect_to_peer(&self, addr: SocketAddr) -> Result<(), NodeError> {
        println!("Attempting to connect to peer at {}", addr);

        match TcpStream::connect(addr).await {
            Ok(stream) => {
                let hello = Message::Hello {
                    address: self.address,
                    node_type: self.node_type.clone(),
                    network: self.network.to_string(),
                };
                let msg = serde_json::to_string(&hello)?;
                let mut stream = tokio::io::BufWriter::new(stream);
                stream.write_all(msg.as_bytes()).await?;
                stream.write_all(b"\n").await?;
                println!("Successfully connected to peer at {}", addr);
                Ok(())
            }
            Err(e) => {
                eprintln!("Failed to connect to peer at {}: {}", addr, e);
                Err(NodeError::P2PError(format!(
                    "Failed to connect to peer at {}: {}",
                    addr, e
                )))
            }
        }
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
    ) -> Result<(), NodeError> {
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

                    self.db.save_blocks(&blockchain.lock().await.blocks)?;

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

    pub async fn get_peers(&self, addr: SocketAddr) -> Result<Vec<SocketAddr>, NodeError> {
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
            Err(NodeError::Other("Invalid response".to_string()))
        }
    }

    pub async fn get_status(&self, addr: SocketAddr) -> Result<(String, u64), NodeError> {
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
            Err(NodeError::Other("Invalid response".to_string()))
        }
    }
}
