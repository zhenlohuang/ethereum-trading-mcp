//! Configuration management module.
//!
//! Handles loading configuration from environment variables.

use std::env;

use crate::error::AppError;
use crate::ethereum::constants::DEFAULT_CHAIN_ID;

/// Application configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// Ethereum JSON-RPC endpoint URL.
    pub rpc_url: String,
    /// Private key for wallet (hex string with 0x prefix).
    pub private_key: String,
    /// Logging level (default: info).
    pub log_level: String,
    /// Chain ID (default: 1 for Ethereum mainnet).
    pub chain_id: u64,
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
    /// - `ETHEREUM_CHAIN_ID`: Chain ID (default: 1 for Ethereum mainnet)
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

        let chain_id = env::var("ETHEREUM_CHAIN_ID")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(DEFAULT_CHAIN_ID);

        Ok(Self { rpc_url, private_key, log_level, chain_id })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Tests that modify environment variables are skipped because they
    // interfere with parallel test execution. Config::from_env() is tested
    // through integration tests instead.

    #[test]
    fn test_config_struct_creation() {
        let config = Config {
            rpc_url: "https://rpc.example.com".to_string(),
            private_key: "0xkey".to_string(),
            log_level: "info".to_string(),
            chain_id: 1,
        };

        assert_eq!(config.rpc_url, "https://rpc.example.com");
        assert_eq!(config.private_key, "0xkey");
        assert_eq!(config.log_level, "info");
        assert_eq!(config.chain_id, 1);
    }

    #[test]
    fn test_config_clone() {
        let config = Config {
            rpc_url: "https://rpc.example.com".to_string(),
            private_key: "0xkey".to_string(),
            log_level: "info".to_string(),
            chain_id: 1,
        };

        let cloned = config.clone();
        assert_eq!(cloned.rpc_url, config.rpc_url);
        assert_eq!(cloned.private_key, config.private_key);
        assert_eq!(cloned.log_level, config.log_level);
        assert_eq!(cloned.chain_id, config.chain_id);
    }

    #[test]
    fn test_config_debug() {
        let config = Config {
            rpc_url: "https://rpc.example.com".to_string(),
            private_key: "0xsecret".to_string(),
            log_level: "warn".to_string(),
            chain_id: 1,
        };

        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("Config"));
        assert!(debug_str.contains("rpc_url"));
        assert!(debug_str.contains("https://rpc.example.com"));
        assert!(debug_str.contains("log_level"));
        assert!(debug_str.contains("chain_id"));
    }

    #[test]
    fn test_default_chain_id_is_mainnet() {
        assert_eq!(DEFAULT_CHAIN_ID, 1);
    }

    #[test]
    fn test_config_with_various_chain_ids() {
        // Mainnet
        let mainnet = Config {
            rpc_url: "https://mainnet.example.com".to_string(),
            private_key: "0x1".to_string(),
            log_level: "info".to_string(),
            chain_id: 1,
        };
        assert_eq!(mainnet.chain_id, 1);

        // Sepolia
        let sepolia = Config {
            rpc_url: "https://sepolia.example.com".to_string(),
            private_key: "0x2".to_string(),
            log_level: "debug".to_string(),
            chain_id: 11155111,
        };
        assert_eq!(sepolia.chain_id, 11155111);

        // Arbitrum
        let arbitrum = Config {
            rpc_url: "https://arbitrum.example.com".to_string(),
            private_key: "0x3".to_string(),
            log_level: "error".to_string(),
            chain_id: 42161,
        };
        assert_eq!(arbitrum.chain_id, 42161);
    }

    #[test]
    fn test_config_log_levels() {
        for level in ["trace", "debug", "info", "warn", "error"] {
            let config = Config {
                rpc_url: "https://rpc.example.com".to_string(),
                private_key: "0x".to_string(),
                log_level: level.to_string(),
                chain_id: 1,
            };
            assert_eq!(config.log_level, level);
        }
    }

    #[test]
    fn test_config_various_rpc_urls() {
        let urls = [
            "https://mainnet.infura.io/v3/YOUR-PROJECT-ID",
            "https://eth-mainnet.g.alchemy.com/v2/YOUR-API-KEY",
            "http://localhost:8545",
            "wss://mainnet.infura.io/ws/v3/YOUR-PROJECT-ID",
        ];

        for url in urls {
            let config = Config {
                rpc_url: url.to_string(),
                private_key: "0x".to_string(),
                log_level: "info".to_string(),
                chain_id: 1,
            };
            assert_eq!(config.rpc_url, url);
        }
    }

    #[test]
    fn test_config_private_key_formats() {
        // With 0x prefix
        let config1 = Config {
            rpc_url: "https://rpc.example.com".to_string(),
            private_key: "0x1234567890abcdef".to_string(),
            log_level: "info".to_string(),
            chain_id: 1,
        };
        assert!(config1.private_key.starts_with("0x"));

        // Without prefix (some tools strip it)
        let config2 = Config {
            rpc_url: "https://rpc.example.com".to_string(),
            private_key: "1234567890abcdef".to_string(),
            log_level: "info".to_string(),
            chain_id: 1,
        };
        assert!(!config2.private_key.starts_with("0x"));
    }
}
