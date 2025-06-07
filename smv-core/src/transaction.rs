use crate::crypto::{Address, Keypair, hash, public_key_to_address, verify};
use crate::state::State;
use crate::{BlockchainError, Result};
use ed25519_dalek::ed25519::signature::SignerMut;
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

#[derive(Clone, Deserialize, Serialize)]
pub struct Transaction {
    pub sender: Address,
    pub receiver: Address,
    pub amount: u64,
    pub nonce: u64,
    #[serde(with = "BigArray")]
    pub signature: [u8; 64],
    pub sender_public_key: [u8; 32],
}

impl std::fmt::Debug for Transaction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Tx {{ from: {:?}, to: {:?}, amount: {}, nonce: {} }}",
            &self.sender, &self.receiver, self.amount, self.nonce
        )
    }
}

pub enum ValidationLevel {
    Light,
    Full,
}

impl Transaction {
    pub fn new(sender_keypair: &Keypair, receiver: Address, amount: u64, nonce: u64) -> Self {
        let sender = public_key_to_address(&sender_keypair.verifying_key);
        let message = Self::create_message(&sender, &receiver, amount, nonce);
        let message_hash = hash(&message);

        // clone is fine if you're using a Copy-safe key or keypool
        let signature = sender_keypair
            .signing_key
            .clone()
            .sign(&message_hash)
            .to_bytes();

        Self {
            sender,
            receiver,
            amount,
            nonce,
            signature,
            sender_public_key: sender_keypair.verifying_key.to_bytes(),
        }
    }

    fn create_message(sender: &Address, receiver: &Address, amount: u64, nonce: u64) -> Vec<u8> {
        let mut message = Vec::with_capacity(20 + 20 + 8 + 8);
        message.extend_from_slice(sender);
        message.extend_from_slice(receiver);
        message.extend_from_slice(&amount.to_le_bytes());
        message.extend_from_slice(&nonce.to_le_bytes());
        message
    }

    pub fn verify(&self) -> Result<()> {
        let public_key = VerifyingKey::from_bytes(&self.sender_public_key)
            .map_err(|_| BlockchainError::InvalidSignature)?;

        // Ensure public key matches declared sender
        let expected_sender = public_key_to_address(&public_key);
        if expected_sender != self.sender {
            return Err(BlockchainError::InvalidSenderAddress);
        }

        let message = Self::create_message(&self.sender, &self.receiver, self.amount, self.nonce);
        let message_hash = hash(&message);
        let signature = Signature::from_bytes(&self.signature);

        verify(&public_key, &message_hash, &signature)
    }

    pub fn hash(&self) -> crate::crypto::Hash {
        let mut bytes = Vec::with_capacity(20 + 20 + 8 + 8);
        bytes.extend_from_slice(&self.sender);
        bytes.extend_from_slice(&self.receiver);
        bytes.extend_from_slice(&self.amount.to_le_bytes());
        bytes.extend_from_slice(&self.nonce.to_le_bytes());
        hash(&bytes)
    }

    pub fn validate(&self, level: ValidationLevel, state: Option<&State>) -> Result<()> {
        match level {
            ValidationLevel::Light => self.validate_light(),
            ValidationLevel::Full => self.validate_full(state.ok_or_else(|| {
                BlockchainError::StateError("State required for full validation".into())
            })?),
        }
    }

    fn validate_light(&self) -> Result<()> {
        self.verify()
    }

    fn validate_full(&self, state: &State) -> Result<()> {
        self.validate_light()?;

        let sender_balance = state.get_balance(&self.sender);
        if sender_balance < self.amount {
            return Err(BlockchainError::InsufficientBalance);
        }

        let current_nonce = state.get_nonce(&self.sender);
        if self.nonce != current_nonce + 1 {
            return Err(BlockchainError::InvalidNonce);
        }

        Ok(())
    }
}
