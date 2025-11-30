//! Smart contract bindings.

pub mod chainlink;
pub mod erc20;
pub mod uniswap_v2;
pub mod uniswap_v3;

use alloy::primitives::{address, Address};

// ============================================================================
// Core Token Addresses (Ethereum Mainnet)
// ============================================================================

/// Wrapped Ether (WETH) address.
pub const WETH_ADDRESS: Address = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");

/// USDC address.
pub const USDC_ADDRESS: Address = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");

/// WBTC address.
pub const WBTC_ADDRESS: Address = address!("2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599");
