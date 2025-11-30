//! Ethereum network constants.
//!
//! Contains chain IDs and mainnet contract addresses.

use alloy::primitives::{address, Address};

// ============================================================================
// Chain IDs
// ============================================================================

/// Ethereum Mainnet chain ID.
pub const ETHEREUM_MAINNET_CHAIN_ID: u64 = 1;

/// Sepolia testnet chain ID.
pub const SEPOLIA_CHAIN_ID: u64 = 11155111;

/// Default chain ID (Ethereum Mainnet).
pub const DEFAULT_CHAIN_ID: u64 = ETHEREUM_MAINNET_CHAIN_ID;

// ============================================================================
// Core Token Addresses (Ethereum Mainnet)
// ============================================================================

/// Wrapped Ether (WETH) address on Ethereum Mainnet.
pub const WETH_ADDRESS: Address = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");

/// USDC address on Ethereum Mainnet.
pub const USDC_ADDRESS: Address = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");

/// WBTC address on Ethereum Mainnet.
pub const WBTC_ADDRESS: Address = address!("2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599");

/// UNI token address on Ethereum Mainnet.
pub const UNI_ADDRESS: Address = address!("1f9840a85d5aF5bf1D1762F925BDADdC4201F984");

// ============================================================================
// Chainlink Price Feed Addresses (Ethereum Mainnet)
// ============================================================================

/// Chainlink ETH/USD price feed address on Ethereum Mainnet.
pub const ETH_USD_FEED: Address = address!("5f4eC3Df9cbd43714FE2740f5E3616155c5b8419");

/// Chainlink BTC/USD price feed address on Ethereum Mainnet.
pub const BTC_USD_FEED: Address = address!("F4030086522a5bEEa4988F8cA5B36dbC97BeE88c");

/// Chainlink USDC/USD price feed address on Ethereum Mainnet.
pub const USDC_USD_FEED: Address = address!("8fFfFfd4AfB6115b954Bd326cbe7B4BA576818f6");

// ============================================================================
// Uniswap V2 Addresses (Ethereum Mainnet)
// ============================================================================

/// Uniswap V2 Router address on Ethereum Mainnet.
pub const UNISWAP_V2_ROUTER: Address = address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D");

/// Uniswap V2 Factory address on Ethereum Mainnet.
pub const UNISWAP_V2_FACTORY: Address = address!("5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f");

// ============================================================================
// Uniswap V3 Addresses (Ethereum Mainnet)
// ============================================================================

/// Uniswap V3 SwapRouter address on Ethereum Mainnet.
pub const UNISWAP_V3_ROUTER: Address = address!("E592427A0AEce92De3Edee1F18E0157C05861564");

/// Uniswap V3 Factory address on Ethereum Mainnet.
pub const UNISWAP_V3_FACTORY: Address = address!("1F98431c8aD98523631AE4a59f267346ea31F984");

/// Uniswap V3 Quoter V2 address on Ethereum Mainnet.
pub const UNISWAP_V3_QUOTER: Address = address!("61fFE014bA17989E743c5F6cB21bF9697530B21e");
