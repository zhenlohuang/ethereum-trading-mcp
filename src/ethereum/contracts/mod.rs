//! Smart contract bindings.

pub mod chainlink;
pub mod erc20;
pub mod uniswap_v2;
pub mod uniswap_v3;

// Re-export core token addresses from constants module.
pub use super::constants::{USDC_ADDRESS, WBTC_ADDRESS, WETH_ADDRESS};
