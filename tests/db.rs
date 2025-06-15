use smvblock::blockchain::{Block, Transaction, User};
use smvblock::db::Database;

#[test]
fn test_add_and_get_block() {
    let mut db = Database::new(None, true).unwrap();

    let block = Block {
        previous_hash: [0; 32],
        merkle_root: [1; 32],
        nonce: 1,
        timestamp: 1234567890,
        transactions: vec![],
    };

    db.add_block(&block).unwrap();
    let fetched_block = db.get_block(&block.previous_hash).unwrap();

    assert!(fetched_block.is_some());
    assert_eq!(fetched_block.unwrap().previous_hash, block.previous_hash);
}

#[test]
fn test_add_and_get_transaction() {
    let db = Database::new(None, true).unwrap();

    let transaction = Transaction {
        sender: [1; 32],
        receiver: [2; 32],
        amount: 100,
        nonce: 0,
        signature: [0; 64],
        sender_public_key: [1; 32],
    };

    db.add_transaction(&transaction).unwrap();
    let transactions = db.get_transactions().unwrap();

    assert_eq!(transactions.len(), 1);
    assert_eq!(transactions[0].sender, transaction.sender);
}

#[test]
fn test_add_and_get_user() {
    let db = Database::new(None, true).unwrap();

    let user = User {
        address: [1; 32],
        public_key: [1; 32],
        balance: 100,
        stake: 0,
    };

    db.add_user(&user).unwrap();
    let users = db.get_users().unwrap();

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].address, user.address);
}
