use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Debug)]
pub enum InterfaceError {
    IoError(std::io::Error),
    SerializationError(serde_json::Error),
    InvalidResponse,
}

impl From<std::io::Error> for InterfaceError {
    fn from(err: std::io::Error) -> Self {
        InterfaceError::IoError(err)
    }
}

impl From<serde_json::Error> for InterfaceError {
    fn from(err: serde_json::Error) -> Self {
        InterfaceError::SerializationError(err)
    }
}

impl std::fmt::Display for InterfaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InterfaceError::IoError(msg) => write!(f, "IO Error: {}", msg),
            InterfaceError::SerializationError(msg) => write!(f, "Serialization Error: {}", msg),
            InterfaceError::InvalidResponse => write!(f, "Invalid Response"),
        }
    }
}

impl std::error::Error for InterfaceError {}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Message {
    Hello {
        address: SocketAddr,
        node_type: String,
        network: String,
    },
    HelloResponse {
        node_type: String,
    },
    GetStatus,
    Status {
        head_hash: String,
        height: u64,
    },
    GetPeers,
    Peers {
        peers: Vec<String>,
    },
    SendTransaction {
        to: String,
        amount: u64,
    },
    TransactionResponse {
        result: Result<String, String>,
    },
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ResponseMessage {
    Status { head_hash: String, height: u64 },
    TransactionResponse { hash: String },
    Peers { peers: Vec<String> },
    HelloResponse { node_type: String },
}

pub async fn send_and_receive_message<T: for<'de> Deserialize<'de>>(
    addr: SocketAddr,
    message: &Message,
) -> Result<T, InterfaceError> {
    let stream = TcpStream::connect(addr).await?;
    let (read_half, write_half) = stream.into_split();
    let mut reader = tokio::io::BufReader::new(read_half);
    let mut writer = tokio::io::BufWriter::new(write_half);

    let serialized_message = serde_json::to_string(message)?;
    writer.write_all(serialized_message.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;

    let mut line = String::new();
    reader.read_line(&mut line).await?;
    let response = serde_json::from_str(&line)?;
    Ok(response)
}

pub async fn handshake(
    addr: SocketAddr,
    local_addr: SocketAddr,
    node_type: String,
    network: String,
) -> Result<String, InterfaceError> {
    let message = Message::Hello {
        address: local_addr,
        node_type,
        network,
    };
    let response: Message = send_and_receive_message(addr, &message).await?;
    if let Message::HelloResponse { node_type } = response {
        Ok(node_type)
    } else {
        Err(InterfaceError::InvalidResponse)
    }
}
