use thiserror::Error;

pub mod block;
pub mod blockchain;
pub mod crypto;
pub mod state;
pub mod transaction;

#[derive(Error, Debug)]
pub enum BlockchainError {
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Invalid hash")]
    InvalidHash,
    #[error("Insufficient balance")]
    InsufficientBalance,
    #[error("Invalid nonce")]
    InvalidNonce,
    #[error("Invalid proof of work")]
    InvalidProofOfWork,
    #[error("Invalid sender address")]
    InvalidSenderAddress,
    #[error("Crypto error: {0}")]
    CryptoError(String),
    #[error("State error: {0}")]
    StateError(String),
}

pub type Result<T> = std::result::Result<T, BlockchainError>;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
