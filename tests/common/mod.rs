//! Common utilities for integration tests.

use ethereum_trading_mcp::{Config, EthereumTradingServer};

/// Helper to create a test server from environment variables.
pub fn create_test_server() -> Option<EthereumTradingServer> {
    // Load .env file if present
    let _ = dotenvy::dotenv();

    // Check if required environment variables are set
    let rpc_url = std::env::var("ETHEREUM_RPC_URL").ok()?;
    let private_key = std::env::var("ETHEREUM_PRIVATE_KEY").ok()?;

    if rpc_url.is_empty() || private_key.is_empty() {
        return None;
    }

    let config = Config { rpc_url, private_key, log_level: "warn".to_string() };

    EthereumTradingServer::new(config).ok()
}

/// Skip test if server cannot be created (missing env vars).
#[macro_export]
macro_rules! skip_if_no_server {
    () => {
        match common::create_test_server() {
            Some(server) => server,
            None => {
                eprintln!("Skipping test: ETHEREUM_RPC_URL or ETHEREUM_PRIVATE_KEY not set");
                return;
            }
        }
    };
}
