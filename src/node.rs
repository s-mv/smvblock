use crate::blockchain::Blockchain;
use crate::db::Database;
use crate::p2p::P2P;
use libp2p::futures::lock::Mutex;
use std::sync::Arc;

pub enum NodeType {
    FullNode,
    LightNode,
}

pub struct Node {
    node_type: NodeType,
    blockchain: Blockchain,
    p2p: P2P,
    database: Arc<Mutex<Database>>,
}

impl Node {
    pub fn new(node_type: NodeType, test_node: bool) -> Result<Self, String> {
        let db = Database::new(None, test_node).map_err(|_| "Failed to initialize database".to_string())?;
        let db = Arc::new(Mutex::new(db));

        Ok(Node {
            node_type,
            blockchain: Blockchain::new(db.clone()),
            p2p: P2P::new(db.clone()),
            database: db,
        })
    }
}
