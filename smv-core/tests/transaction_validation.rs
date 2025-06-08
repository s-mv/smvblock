use smv_core::crypto::{generate_keypair, public_key_to_address};
use smv_core::state::State;
use smv_core::transaction::{Transaction, ValidationLevel};

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

fn setup_transaction() -> (Transaction, State) {
    let sender_keypair = generate_keypair();
    let receiver_keypair = generate_keypair();
    let sender_address = public_key_to_address(&sender_keypair.verifying_key);
    let receiver_address = public_key_to_address(&receiver_keypair.verifying_key);

    let mut state = State::new();
    state.set_balance(&sender_address, 1000);
    state.set_nonce(&sender_address, 0);

    let transaction = Transaction::new(&sender_keypair, receiver_address, 500, 1);

    (transaction, state)
}

#[test]
fn test_light_validation_success() {
    let (transaction, _) = setup_transaction();
    assert!(transaction.validate(ValidationLevel::Light, None).is_ok());
}

#[test]
fn test_light_validation_invalid_signature() {
    let (mut transaction, _) = setup_transaction();
    // corrupt signature
    transaction.signature[0] = !transaction.signature[0];
    assert!(transaction.validate(ValidationLevel::Light, None).is_err());
}

#[test]
fn test_full_validation_success() {
    let (transaction, state) = setup_transaction();
    assert!(
        transaction
            .validate(ValidationLevel::Full, Some(&state))
            .is_ok()
    );
}

#[test]
fn test_full_validation_insufficient_balance() {
    let (transaction, mut state) = setup_transaction();
    let sender_address = transaction.sender;
    state.set_balance(&sender_address, 100);
    assert!(
        transaction
            .validate(ValidationLevel::Full, Some(&state))
            .is_err()
    );
}

#[test]
fn test_full_validation_invalid_nonce() {
    let (transaction, mut state) = setup_transaction();
    let sender_address = transaction.sender;
    state.set_nonce(&sender_address, 5);
    assert!(
        transaction
            .validate(ValidationLevel::Full, Some(&state))
            .is_err()
    );
}

#[test]
fn test_full_validation_without_state() {
    let (transaction, _) = setup_transaction();
    assert!(transaction.validate(ValidationLevel::Full, None).is_err());
}
