pub mod blockchain;
pub mod db;
pub mod node;
pub mod p2p;

pub fn main() {
    let node = node::Node::new(node::NodeType::FullNode, true).expect("Failed to initialize node");

    println!("Node initialized!");
}
