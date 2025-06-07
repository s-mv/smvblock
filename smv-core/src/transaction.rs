use crate::crypto::{Address, Keypair, hash, verify};
use crate::{BlockchainError, Result};
use ed25519_dalek::ed25519::signature::SignerMut;
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Transaction {
    pub sender: Address,
    pub receiver: Address,
    pub amount: u64,
    pub nonce: u64,
    #[serde(with = "BigArray")]
    pub signature: [u8; 64],
    pub sender_public_key: [u8; 32],
}

impl Transaction {
    pub fn new(sender_keypair: &Keypair, receiver: Address, amount: u64, nonce: u64) -> Self {
        let sender = crate::crypto::public_key_to_address(&sender_keypair.verifying_key);
        let message = Self::create_message(&sender, &receiver, amount, nonce);
        let signature = sender_keypair.signing_key.clone().sign(&message);
        let signature = signature.to_bytes();

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
        let mut message = Vec::new();
        message.extend_from_slice(sender);
        message.extend_from_slice(receiver);
        message.extend_from_slice(&amount.to_le_bytes());
        message.extend_from_slice(&nonce.to_le_bytes());
        message
    }

    pub fn verify(&self) -> Result<()> {
        let public_key = VerifyingKey::from_bytes(&self.sender_public_key)
            .map_err(|_| BlockchainError::InvalidSignature)?;

        let message = Self::create_message(&self.sender, &self.receiver, self.amount, self.nonce);
        let signature = Signature::from_bytes(&self.signature);

        verify(&public_key, &message, &signature)
    }

    pub fn hash(&self) -> crate::crypto::Hash {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.sender);
        bytes.extend_from_slice(&self.receiver);
        bytes.extend_from_slice(&self.amount.to_le_bytes());
        bytes.extend_from_slice(&self.nonce.to_le_bytes());
        hash(&bytes)
    }
}
