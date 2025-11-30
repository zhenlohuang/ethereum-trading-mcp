//! Smart contract bindings.

pub mod chainlink;
pub mod erc20;
pub mod uniswap_v2;
pub mod uniswap_v3;

use alloy::primitives::{address, Address};

// ============================================================================
// Common Token Addresses (Ethereum Mainnet) - Static Fallback
// ============================================================================
//
// NOTE: For production use, prefer `TokenRegistry` from `crate::services::token_registry`
// which provides:
//   - Dynamic token list fetching from Uniswap Token Lists
//   - In-memory caching with TTL
//   - Multi-chain support
//   - More comprehensive token coverage
//
// These static constants are kept as compile-time fallbacks for core tokens.
// ============================================================================

/// Wrapped Ether (WETH) address.
pub const WETH_ADDRESS: Address = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");

/// USDC address.
pub const USDC_ADDRESS: Address = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");

/// USDT address.
pub const USDT_ADDRESS: Address = address!("dAC17F958D2ee523a2206206994597C13D831ec7");

/// DAI address.
pub const DAI_ADDRESS: Address = address!("6B175474E89094C44Da98b954EecdeCB5BadD191");

/// WBTC address.
pub const WBTC_ADDRESS: Address = address!("2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599");

/// LINK (Chainlink) address.
pub const LINK_ADDRESS: Address = address!("514910771AF9Ca656af840dff83E8264EcF986CA");

/// UNI (Uniswap) address.
pub const UNI_ADDRESS: Address = address!("1f9840a85d5aF5bf1D1762F925BDADdC4201F984");

// ============================================================================
// Token Symbol Resolution (Static Fallback)
// ============================================================================

/// Resolve a token symbol to an Address using static fallback data.
///
/// **Note**: For production use, prefer `TokenRegistry::resolve_symbol()` which
/// provides dynamic token list support with caching.
///
/// Supports common token symbols (case-insensitive):
/// - WETH, ETH -> Wrapped Ether
/// - USDC -> USD Coin
/// - USDT, TETHER -> Tether
/// - DAI -> Dai Stablecoin
/// - WBTC -> Wrapped Bitcoin
/// - LINK -> Chainlink
/// - UNI -> Uniswap
///
/// Returns `None` if the symbol is not recognized.
pub fn resolve_token_symbol(symbol: &str) -> Option<Address> {
    match symbol.to_uppercase().as_str() {
        "WETH" | "ETH" => Some(WETH_ADDRESS),
        "USDC" => Some(USDC_ADDRESS),
        "USDT" | "TETHER" => Some(USDT_ADDRESS),
        "DAI" => Some(DAI_ADDRESS),
        "WBTC" => Some(WBTC_ADDRESS),
        "LINK" | "CHAINLINK" => Some(LINK_ADDRESS),
        "UNI" | "UNISWAP" => Some(UNI_ADDRESS),
        _ => None,
    }
}
