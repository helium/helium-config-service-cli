use solana_client::pubsub_client::PubsubClientError;
use solana_sdk::{program_error::ProgramError, pubkey::ParsePubkeyError};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("RPC error: {0}")]
    RpcError(Box<solana_client::client_error::ClientError>),
    #[error("Failed to parse bincode: {0}")]
    ParseBincodeError(#[from] Box<bincode::ErrorKind>),
    #[error("Failed to parse pubkey: {0}")]
    ParsePubkeyError(#[from] ParsePubkeyError),
    #[error("Anchor error: {0}")]
    AnchorError(#[from] anchor_lang::error::Error),
    #[error("Solana Pubsub error: {0}")]
    SolanaPubsubError(#[from] PubsubClientError),
    #[error("Program error: {0}")]
    ProgramError(#[from] ProgramError),
    #[error("Account required for the instruction was not found")]
    AccountNotFound(String),
    #[error("Organization already exists")]
    OrganizationAlreadyExists(String),
    #[error("Devaddr Constraint already initialized")]
    DevaddrConstraintAlreadyInitialized,
}

impl From<solana_client::client_error::ClientError> for Error {
    fn from(value: solana_client::client_error::ClientError) -> Self {
        Self::RpcError(Box::new(value))
    }
}
