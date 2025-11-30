//! Error types and handling module.
//!
//! Defines all application-specific error types and conversions.

use alloy::primitives::Address;
use rmcp::ErrorData as McpError;
use thiserror::Error;

/// Application-wide error type.
#[derive(Debug, Error)]
pub enum AppError {
    /// Configuration-related errors.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Ethereum RPC errors.
    #[error("Ethereum RPC error: {0}")]
    Rpc(String),

    /// Transport errors.
    #[error("Transport error: {0}")]
    Transport(String),

    /// Invalid Ethereum address.
    #[error("Invalid address: {0}")]
    InvalidAddress(String),

    /// Token not found or invalid.
    #[error("Token not found: {0}")]
    TokenNotFound(Address),

    /// Insufficient liquidity for swap.
    #[error("Insufficient liquidity for swap")]
    InsufficientLiquidity,

    /// Slippage exceeded.
    #[error("Slippage too high: expected {expected}, got {actual}")]
    SlippageExceeded { expected: String, actual: String },

    /// Wallet-related errors.
    #[error("Wallet error: {0}")]
    Wallet(String),

    /// Simulation failed.
    #[error("Simulation failed: {0}")]
    SimulationFailed(String),

    /// Pool not found.
    #[error("Pool not found for token pair")]
    PoolNotFound,

    /// Parse error.
    #[error("Parse error: {0}")]
    Parse(String),

    /// Price oracle failure (e.g., stale or invalid data).
    #[error("Price oracle error: {0}")]
    PriceOracle(String),

    /// Numeric overflow during conversion.
    #[error("Numeric overflow: {0}")]
    NumericOverflow(String),

    /// Pending transaction error.
    #[error("Pending transaction error: {0}")]
    PendingTransaction(String),
}

impl From<alloy::transports::TransportError> for AppError {
    fn from(err: alloy::transports::TransportError) -> Self {
        AppError::Transport(err.to_string())
    }
}

impl From<alloy::contract::Error> for AppError {
    fn from(err: alloy::contract::Error) -> Self {
        AppError::Rpc(err.to_string())
    }
}

impl From<alloy::signers::local::LocalSignerError> for AppError {
    fn from(err: alloy::signers::local::LocalSignerError) -> Self {
        AppError::Wallet(err.to_string())
    }
}

impl From<alloy::hex::FromHexError> for AppError {
    fn from(err: alloy::hex::FromHexError) -> Self {
        AppError::Parse(err.to_string())
    }
}

impl From<std::num::ParseIntError> for AppError {
    fn from(err: std::num::ParseIntError) -> Self {
        AppError::Parse(err.to_string())
    }
}

impl From<AppError> for McpError {
    fn from(err: AppError) -> Self {
        match err {
            AppError::InvalidAddress(_)
            | AppError::TokenNotFound(_)
            | AppError::Parse(_)
            | AppError::NumericOverflow(_) => McpError::invalid_params(err.to_string(), None),
            AppError::Config(_) => McpError::invalid_request(err.to_string(), None),
            _ => McpError::internal_error(err.to_string(), None),
        }
    }
}

/// Result type alias using AppError.
pub type Result<T> = std::result::Result<T, AppError>;
