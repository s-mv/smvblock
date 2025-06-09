use anyhow::{Context, Result};
use serde::Deserialize;
use smv_core::interface::{Message, ResponseMessage};
use std::net::SocketAddr;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use url::Url;

#[derive(Debug, Deserialize)]
pub struct NodeStatus {
    pub head_hash: String,
    pub height: u64,
}

pub async fn get_status(node: &Url) -> Result<NodeStatus> {
    let addr = format!(
        "{}:{}",
        node.host_str().unwrap(),
        node.port().unwrap_or(8000)
    );
    let stream = TcpStream::connect(addr)
        .await
        .context("Failed to connect to node")?;
    let (read_half, write_half) = stream.into_split();
    let mut reader = tokio::io::BufReader::new(read_half);
    let mut writer = tokio::io::BufWriter::new(write_half);

    let msg = serde_json::to_string(&Message::GetStatus)?;
    writer.write_all(msg.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;

    let mut line = String::new();
    reader.read_line(&mut line).await?;

    let msg: ResponseMessage = serde_json::from_str(&line)?;
    match msg {
        ResponseMessage::Status { head_hash, height } => Ok(NodeStatus { head_hash, height }),
        _ => Err(anyhow::anyhow!("Unexpected response type")),
    }
}

pub async fn send_transaction(node: &Url, to: &str, amount: u64) -> Result<String> {
    let addr = format!(
        "{}:{}",
        node.host_str().unwrap(),
        node.port().unwrap_or(8000)
    );
    let stream = TcpStream::connect(addr)
        .await
        .context("Failed to connect to node")?;
    let (read_half, write_half) = stream.into_split();
    let mut reader = tokio::io::BufReader::new(read_half);
    let mut writer = tokio::io::BufWriter::new(write_half);

    let msg = serde_json::to_string(&Message::SendTransaction {
        to: to.to_string(),
        amount,
    })?;
    writer.write_all(msg.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;

    let mut line = String::new();
    reader.read_line(&mut line).await?;

    if let Ok(msg) = serde_json::from_str::<ResponseMessage>(&line) {
        match msg {
            ResponseMessage::TransactionResponse { hash } => Ok(hash),
            _ => Err(anyhow::anyhow!("Unexpected response type")),
        }
    } else if let Ok(msg) = serde_json::from_str::<Message>(&line) {
        match msg {
            Message::TransactionResponse { result } => match result {
                Ok(hash) => Ok(hash),
                Err(error) => Err(anyhow::anyhow!("Transaction failed: {}", error)),
            },
            _ => Err(anyhow::anyhow!("Unexpected response type")),
        }
    } else {
        Err(anyhow::anyhow!("Failed to parse response"))
    }
}

pub async fn get_peers(node: &Url) -> Result<Vec<SocketAddr>> {
    let addr = format!(
        "{}:{}",
        node.host_str().unwrap(),
        node.port().unwrap_or(8000)
    );
    let stream = TcpStream::connect(addr)
        .await
        .context("Failed to connect to node")?;
    let (read_half, write_half) = stream.into_split();
    let mut reader = tokio::io::BufReader::new(read_half);
    let mut writer = tokio::io::BufWriter::new(write_half);

    let msg = serde_json::to_string(&Message::GetPeers)?;
    writer.write_all(msg.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;

    let mut line = String::new();
    reader.read_line(&mut line).await?;

    let msg: ResponseMessage = serde_json::from_str(&line)?;

    if let ResponseMessage::Peers { peers } = msg {
        let socket_addrs: Result<Vec<SocketAddr>, _> =
            peers.into_iter().map(|s| s.parse::<SocketAddr>()).collect();
        let socket_addrs = socket_addrs.context("Failed to parse peer address")?;

        println!("Deserialized peers list: {:?}", socket_addrs);
        Ok(socket_addrs)
    } else {
        Err(anyhow::anyhow!("Unexpected response type"))
    }
}

pub async fn connect_to_peer(node: &Url, peer: &str) -> Result<String> {
    let addr = format!(
        "{}:{}",
        node.host_str().unwrap(),
        node.port().unwrap_or(8000)
    );
    let stream = TcpStream::connect(addr)
        .await
        .context("Failed to connect to node")?;
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = tokio::io::BufReader::new(read_half);

    let peer_addr: SocketAddr = peer.parse()?;
    let msg = serde_json::to_string(&Message::Hello {
        address: peer_addr,
        node_type: "Unknown".to_string(),
        network: node.scheme().to_string(),
    })?;
    write_half.write_all(msg.as_bytes()).await?;
    write_half.write_all(b"\n").await?;
    write_half.flush().await?;

    let mut line = String::new();
    reader.read_line(&mut line).await?;
    let response: ResponseMessage = serde_json::from_str(&line)?;
    if let ResponseMessage::HelloResponse { node_type } = response {
        Ok(node_type)
    } else {
        Err(anyhow::anyhow!("Unexpected response type"))
    }
}
