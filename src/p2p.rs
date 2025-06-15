use crate::db::Database;
use libp2p::futures::lock::Mutex;
use std::sync::Arc;

pub struct P2P {
    db: Arc<Mutex<Database>>,
}

impl P2P {
    pub fn new(db: Arc<Mutex<Database>>) -> Self {
        P2P { db }
    }
}
