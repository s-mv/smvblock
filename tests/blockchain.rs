use libp2p::futures::lock::Mutex;
use smvblock::blockchain::{Address, Block, Blockchain, Transaction, User};
use smvblock::db::Database;
use std::sync::Arc;

#[tokio::test]
async fn test_add_block() {
    let db = Arc::new(Mutex::new(Database::new(None, true).unwrap()));
    let mut blockchain = Blockchain::new(db.clone());

    let block = Block::new([0; 32], 1, vec![]);
    let proposer: Address = [1; 32];

    blockchain.add_block(block.clone(), proposer).await.unwrap();
    let fetched_block = blockchain.get_block(block.hash().unwrap()).await.unwrap();

    assert!(fetched_block.is_some());
    assert_eq!(
        fetched_block.unwrap().hash().unwrap(),
        block.hash().unwrap()
    );
}

#[tokio::test]
async fn test_add_transaction() {
    let db = Arc::new(Mutex::new(Database::new(None, true).unwrap()));
    let blockchain = Blockchain::new(db.clone());

    let transaction = Transaction {
        sender: [1; 32],
        receiver: [2; 32],
        amount: 100,
        nonce: 0,
        signature: [0; 64],
        sender_public_key: [1; 32],
    };

    blockchain
        .add_transaction(transaction.clone())
        .await
        .unwrap();
    let transactions = blockchain.get_transactions().await.unwrap();

    assert_eq!(transactions.len(), 1);
    assert_eq!(transactions[0].sender, transaction.sender);
}

#[tokio::test]
async fn test_stake() {
    let db = Arc::new(Mutex::new(Database::new(None, true).unwrap()));
    let blockchain = Blockchain::new(db.clone());

    let user = User {
        address: [1; 32],
        public_key: [1; 32],
        balance: 100,
        stake: 0,
    };

    blockchain.add_user(user.clone()).await.unwrap();
    blockchain.stake(user.address, 50).await.unwrap();

    let users = blockchain.get_users().await.unwrap();
    let updated_user = users.iter().find(|u| u.address == user.address).unwrap();

    assert_eq!(updated_user.balance, 50);
    assert_eq!(updated_user.stake, 50);
}
