use smvblock::{
    blockchain::User,
    node::{Node, NodeType},
};

#[tokio::main]
async fn main() {
    let mut node = Node::new(NodeType::FullNode, true).unwrap();

    let (user1, user1_pk) = User::generate(100);
    let (user2, _) = User::generate(100);

    node.add_user(user1.clone()).await.unwrap();
    node.add_user(user2.clone()).await.unwrap();

    node.stake(user1.address, 30).await.unwrap();
    node.stake(user2.address, 20).await.unwrap();

    let users = node.get_users().await.unwrap();

    for user in users {
        println!(
            "User: {}, Balance: {}, Stake: {}",
            hex::encode(user.address),
            user.balance,
            user.stake
        );
    }

    node.send_transaction(user1_pk, user2.address, 20)
        .await
        .unwrap();

    println!(
        "Transaction sent from {} to {}",
        hex::encode(user1.address),
        hex::encode(user2.address)
    );

    let block_hash = node.produce_block().await.unwrap();
    println!("Produced block with hash: {}", hex::encode(block_hash));

    let users = node.get_users().await.unwrap();

    for user in users {
        println!(
            "User: {}, Balance: {}, Stake: {}",
            hex::encode(user.address),
            user.balance,
            user.stake
        );
    }
}
