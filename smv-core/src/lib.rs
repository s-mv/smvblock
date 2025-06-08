use core::fmt;

use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Network {
    Devnet,
    Mainnet,
}

impl Network {
    pub fn as_str(&self) -> &'static str {
        match self {
            Network::Devnet => "devnet",
            Network::Mainnet => "mainnet",
        }
    }

    pub fn genesis_hash(&self) -> String {
        match self {
            Network::Devnet => "000000dev0000000000000000000000000000000000000000000000000000000",
            Network::Mainnet => "000000main000000000000000000000000000000000000000000000000000000",
        }
        .to_string()
    }
}

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Network::Mainnet => write!(f, "Mainnet"),
            Network::Devnet => write!(f, "Devnet"),
        }
    }
}
