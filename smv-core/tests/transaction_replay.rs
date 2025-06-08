use smv_core::blockchain::Blockchain;
use smv_core::crypto::{generate_keypair, public_key_to_address};
use smv_core::transaction::Transaction;

#[test]
fn reject_duplicate_transaction() {
    let mut blockchain = Blockchain::new();
    let pikachu_keypair = generate_keypair();
    let geodude_keypair = generate_keypair();
    let geodude_address = public_key_to_address(&geodude_keypair.verifying_key);

    blockchain
        .state
        .set_balance(&public_key_to_address(&pikachu_keypair.verifying_key), 1000);

    let tx = Transaction::new(&pikachu_keypair, geodude_address, 10, 1);

    blockchain.add_transaction(tx.clone()).unwrap();
    blockchain.mine_block().unwrap();

    assert!(blockchain.add_transaction(tx).is_err());
}
