use hex;
use smvblock::{
    blockchain::User,
    node::{Node, NodeType},
};

#[tokio::test]
async fn test_basic_flow_transaction_and_block() {
    let mut node = Node::new(NodeType::FullNode, true).unwrap();

    let (user1, user1_pk) = User::generate(100);
    let (user2, _) = User::generate(100);

    node.add_user(user1.clone()).await.unwrap();
    node.add_user(user2.clone()).await.unwrap();

    node.stake(user1.address, 30).await.unwrap();
    node.stake(user2.address, 20).await.unwrap();

    let users = node.get_users().await.unwrap();
    assert_eq!(users.len(), 2);

    let u1 = users.iter().find(|u| u.address == user1.address).unwrap();
    let u2 = users.iter().find(|u| u.address == user2.address).unwrap();
    assert_eq!(u1.stake, 30);
    assert_eq!(u2.stake, 20);

    node.send_transaction(user1_pk.clone(), user2.address, 20)
        .await
        .unwrap();

    let block_hash = node.produce_block().await.unwrap();
    assert_ne!(block_hash, [0u8; 32]);

    let users = node.get_users().await.unwrap();
    let u1 = users.iter().find(|u| u.address == user1.address).unwrap();
    let u2 = users.iter().find(|u| u.address == user2.address).unwrap();

    assert_eq!(u1.balance + u1.stake + u2.balance + u2.stake, 200 + 1);
}

#[tokio::test]
async fn test_transaction_exceeding_balance_fails() {
    let node = Node::new(NodeType::FullNode, true).unwrap();

    let (user1, pk1) = User::generate(100);
    let (user2, _) = User::generate(100);

    node.add_user(user1.clone()).await.unwrap();
    node.add_user(user2.clone()).await.unwrap();

    node.stake(user1.address, 80).await.unwrap();

    let result = node.send_transaction(pk1.clone(), user2.address, 30).await;

    assert!(
        result.is_err(),
        "Transaction should fail due to insufficient balance"
    );

    let result_ok = node.send_transaction(pk1, user2.address, 15).await;
    assert!(
        result_ok.is_ok(),
        "Transaction with valid balance should succeed"
    );
}

#[tokio::test]
async fn test_produce_block_with_no_transactions() {
    let mut node = Node::new(NodeType::FullNode, true).unwrap();

    let (user, _) = User::generate(100);
    node.add_user(user.clone()).await.unwrap();
    node.stake(user.address, 50).await.unwrap();

    let block_hash = node.produce_block().await.unwrap();
    assert_ne!(block_hash, [0u8; 32]); // still produces a block
}
