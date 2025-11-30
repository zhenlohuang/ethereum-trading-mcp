//! Ethereum Trading MCP Server Library
//!
//! A Model Context Protocol server for Ethereum trading operations.
//! Provides tools for querying balances, token prices, and simulating Uniswap swaps.
//!
//! # Features
//!
//! - **Balance Queries**: Query ETH and ERC20 token balances
//! - **Price Lookups**: Get token prices from Chainlink oracles and Uniswap pools
//! - **Swap Simulation**: Simulate Uniswap V2/V3 swaps without on-chain execution
//!
//! # Example
//!
//! ```rust,ignore
//! use ethereum_trading_mcp::{Config, EthereumTradingServer};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = Config::from_env()?;
//!     let server = EthereumTradingServer::new(config).await?;
//!     // Run server...
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod error;
pub mod ethereum;
pub mod mcp;
pub mod services;
pub mod types;

pub use config::Config;
pub use error::{AppError, Result};
pub use ethereum::constants::*;
pub use mcp::EthereumTradingServer;
