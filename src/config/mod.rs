//! Configuration management module.
//!
//! Handles loading configuration from environment variables.

use std::env;

use crate::error::AppError;

/// Application configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// Ethereum JSON-RPC endpoint URL.
    pub rpc_url: String,
    /// Private key for wallet (hex string with 0x prefix).
    pub private_key: String,
    /// Logging level (default: info).
    pub log_level: String,
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// Required environment variables:
    /// - `ETHEREUM_RPC_URL`: Ethereum JSON-RPC endpoint
    /// - `ETHEREUM_PRIVATE_KEY`: Private key for wallet (hex)
    ///
    /// Optional environment variables:
    /// - `LOG_LEVEL`: Logging level (default: info)
    ///
    /// Note: Only Ethereum mainnet (chain ID 1) is currently supported.
    pub fn from_env() -> Result<Self, AppError> {
        // Load .env file if present
        let _ = dotenvy::dotenv();

        let rpc_url = env::var("ETHEREUM_RPC_URL").map_err(|_| {
            AppError::Config("ETHEREUM_RPC_URL environment variable not set".into())
        })?;

        let private_key = env::var("ETHEREUM_PRIVATE_KEY").map_err(|_| {
            AppError::Config("ETHEREUM_PRIVATE_KEY environment variable not set".into())
        })?;

        let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

        Ok(Self { rpc_url, private_key, log_level })
    }
}
