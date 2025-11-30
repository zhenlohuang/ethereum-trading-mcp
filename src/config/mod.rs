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
    /// Ethereum chain ID (default: 1 for mainnet).
    pub chain_id: u64,
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
    /// - `ETHEREUM_CHAIN_ID`: Chain ID (default: 1)
    /// - `LOG_LEVEL`: Logging level (default: info)
    pub fn from_env() -> Result<Self, AppError> {
        // Load .env file if present
        let _ = dotenvy::dotenv();

        let rpc_url = env::var("ETHEREUM_RPC_URL").map_err(|_| {
            AppError::Config("ETHEREUM_RPC_URL environment variable not set".into())
        })?;

        let private_key = env::var("ETHEREUM_PRIVATE_KEY").map_err(|_| {
            AppError::Config("ETHEREUM_PRIVATE_KEY environment variable not set".into())
        })?;

        let chain_id = env::var("ETHEREUM_CHAIN_ID")
            .unwrap_or_else(|_| "1".to_string())
            .parse()
            .map_err(|_| AppError::Config("Invalid ETHEREUM_CHAIN_ID".into()))?;

        let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

        Ok(Self { rpc_url, chain_id, private_key, log_level })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_from_env_missing_rpc_url() {
        // Clear the env var if set
        env::remove_var("ETHEREUM_RPC_URL");
        env::remove_var("ETHEREUM_PRIVATE_KEY");

        let result = Config::from_env();
        assert!(result.is_err());
    }
}
