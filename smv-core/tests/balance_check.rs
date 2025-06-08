use smv_core::blockchain::Blockchain;
use smv_core::crypto::{generate_keypair, public_key_to_address};
use smv_core::transaction::Transaction;

#[test]
fn block_with_insufficient_balance_transaction() {
    let pikachu_keypair = generate_keypair();
    let geodude_keypair = generate_keypair();
    let geodude_address = public_key_to_address(&geodude_keypair.verifying_key);

    let mut blockchain = Blockchain::new();
    println!("geodudea");

    let tx = Transaction::new(&pikachu_keypair, geodude_address, 50, 1);

    assert!(blockchain.add_transaction(tx).is_err());
}
