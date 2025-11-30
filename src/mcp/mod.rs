//! MCP server module.
//!
//! Contains the MCP server implementation with tool handlers.

pub mod server;

pub use server::EthereumTradingServer;
pub use server::{GetBalanceInput, GetTokenPriceInput, SwapTokensInput};
