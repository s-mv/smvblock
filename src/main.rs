use smvblock::blockchain::{User, derive_public_key};
use smvblock::node::{Node, NodeType};

#[tokio::main]
async fn main() {
    let node = Node::new(NodeType::FullNode, true).expect("Failed to initialize node");

    let (user, key) = User::generate(500);

    node.add_user(user).await.unwrap();

    let users = node.get_users().await.unwrap();

    println!("Users in the node: {:?}", users);
    println!("User's private key: {:?}", key);
    println!(
        "User's public key derived from the private key: {:?}",
        derive_public_key(&key)
    );
    println!("Node type: {:?}", node.node_type);
    println!("Node database: {:?}", node.database);
    println!("Node blockchain: {:?}", node.blockchain);
    println!("Node P2P: {:?}", node.p2p);
    println!("Node initialized successfully!");
}
