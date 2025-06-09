use crate::http::{connect_to_peer, get_peers, get_status, send_transaction};
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

pub async fn handle_connect(node: Url, peer: String) -> Result<()> {
    let node_type = connect_to_peer(&node, &peer).await?;
    println!("Successfully connected to peer: {}", peer);
    println!("Peer node type: {}", node_type);
    Ok(())
}

pub async fn handle_get_peers(node: Url) -> Result<()> {
    let peers = get_peers(&node).await?;
    println!("Connected peers:");
    for peer in peers {
        println!("  {}", peer);
    }
    Ok(())
}

pub async fn handle_add_peer(node: Url, peer: String) -> Result<()> {
    connect_to_peer(&node, &peer).await?;
    println!("Successfully added peer: {}", peer);
    Ok(())
}

pub async fn handle_list_peers(node: Url) -> Result<()> {
    let peers = get_peers(&node).await?;
    println!("Connected peers:");
    for peer in peers {
        println!("  {}", peer);
    }
    Ok(())
}
