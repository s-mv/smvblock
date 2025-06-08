use crate::http::{get_status, send_transaction};
use anyhow::Result;
use url::Url;

pub async fn handle_status(node: Url) -> Result<()> {
    let status = get_status(&node).await?;
    println!("Node status:");
    println!("  Head hash: {}", status.head_hash);
    println!("  Height: {}", status.height);
    Ok(())
}

pub async fn handle_send_tx(node: Url, to: String, amount: u64) -> Result<()> {
    let tx_hash = send_transaction(&node, &to, amount).await?;
    println!("Transaction sent successfully!");
    println!("Transaction hash: {}", tx_hash);
    Ok(())
}
