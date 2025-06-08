use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use url::Url;

#[derive(Debug, Deserialize)]
pub struct NodeStatus {
    pub head_hash: String,
    pub height: u64,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum Message {
    GetStatus {},
    SendTransaction { to: String, amount: u64 },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ResponseMessage {
    Status { head_hash: String, height: u64 },
    TransactionResponse { hash: String },
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

    let msg = serde_json::to_string(&Message::GetStatus {})?;
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

    let msg: ResponseMessage = serde_json::from_str(&line)?;
    match msg {
        ResponseMessage::TransactionResponse { hash } => Ok(hash),
        _ => Err(anyhow::anyhow!("Unexpected response type")),
    }
}
