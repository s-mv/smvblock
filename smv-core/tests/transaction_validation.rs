use smv_core::crypto::{generate_keypair, public_key_to_address};
use smv_core::transaction::Transaction;

#[test]
fn create_and_verify_transaction() {
    let pikachu_keypair = generate_keypair();
    let geodude_keypair = generate_keypair();
    let geodude_address = public_key_to_address(&geodude_keypair.verifying_key);

    let tx = Transaction::new(&pikachu_keypair, geodude_address, 100, 1);
    
    assert!(tx.verify().is_ok());
    assert_eq!(tx.amount, 100);
    assert_eq!(tx.receiver, geodude_address);
    assert_eq!(tx.nonce, 1);
}
