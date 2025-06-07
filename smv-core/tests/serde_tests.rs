use smv_core::block::Block;
use smv_core::crypto::{generate_keypair, public_key_to_address};
use smv_core::transaction::Transaction;
use serde_json;

#[test]
fn serialize_and_deserialize_block() {
    let pikachu_keypair = generate_keypair();
    let geodude_keypair = generate_keypair();
    let geodude_address = public_key_to_address(&geodude_keypair.verifying_key);

    let tx = Transaction::new(&pikachu_keypair, geodude_address, 100, 1);
    let block = Block::new(vec![tx], [0; 32]);
    
    let serialized = serde_json::to_string(&block).unwrap();
    let deserialized: Block = serde_json::from_str(&serialized).unwrap();
    
    assert_eq!(block.hash, deserialized.hash);
    assert_eq!(block.previous_hash, deserialized.previous_hash);
    assert_eq!(block.transactions.len(), deserialized.transactions.len());
}
