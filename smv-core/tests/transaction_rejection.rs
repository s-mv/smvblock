use smv_core::crypto::{generate_keypair, public_key_to_address};
use smv_core::transaction::Transaction;

#[test]
fn reject_transaction_with_invalid_signature() {
    let pikachu_keypair = generate_keypair();
    let geodude_keypair = generate_keypair();
    let geodude_address = public_key_to_address(&geodude_keypair.verifying_key);

    let mut tx = Transaction::new(&pikachu_keypair, geodude_address, 100, 1);
    
    tx.signature[0] ^= 0xFF;
    
    assert!(tx.verify().is_err());
}
