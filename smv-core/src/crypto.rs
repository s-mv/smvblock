use crate::BlockchainError;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};

pub type Address = [u8; 32];
pub type Hash = [u8; 32];

pub struct Keypair {
    pub signing_key: SigningKey,
    pub verifying_key: VerifyingKey,
}

pub fn generate_keypair() -> Keypair {
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key = signing_key.verifying_key();
    Keypair {
        signing_key,
        verifying_key,
    }
}

pub fn hash(data: &[u8]) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

pub fn sign(keypair: &Keypair, message: &[u8]) -> Signature {
    keypair.signing_key.sign(message)
}

pub fn verify(
    public_key: &VerifyingKey,
    message: &[u8],
    signature: &Signature,
) -> Result<(), BlockchainError> {
    public_key
        .verify(message, signature)
        .map_err(|_| BlockchainError::InvalidSignature)
}

pub fn public_key_to_address(public_key: &VerifyingKey) -> Address {
    hash(public_key.as_bytes())
}
