//! Chainlink price feed contract bindings.

use alloy::{primitives::address, sol};
use std::collections::HashMap;

use super::{USDC_ADDRESS, WBTC_ADDRESS, WETH_ADDRESS};

/// Chainlink ETH/USD price feed address.
pub const ETH_USD_FEED: alloy::primitives::Address =
    address!("5f4eC3Df9cbd43714FE2740f5E3616155c5b8419");

/// Chainlink BTC/USD price feed address.
pub const BTC_USD_FEED: alloy::primitives::Address =
    address!("F4030086522a5bEEa4988F8cA5B36dbC97BeE88c");

/// Chainlink USDC/USD price feed address.
pub const USDC_USD_FEED: alloy::primitives::Address =
    address!("8fFfFfd4AfB6115b954Bd326cbe7B4BA576818f6");

// Chainlink Aggregator V3 interface
sol! {
    #[sol(rpc)]
    interface IAggregatorV3 {
        function decimals() external view returns (uint8);
        function description() external view returns (string memory);
        function version() external view returns (uint256);

        function latestRoundData()
            external
            view
            returns (
                uint80 roundId,
                int256 answer,
                uint256 startedAt,
                uint256 updatedAt,
                uint80 answeredInRound
            );
    }
}

/// Get known Chainlink price feeds for common tokens.
pub fn get_chainlink_feeds() -> HashMap<alloy::primitives::Address, alloy::primitives::Address> {
    let mut feeds = HashMap::new();
    feeds.insert(WETH_ADDRESS, ETH_USD_FEED);
    feeds.insert(WBTC_ADDRESS, BTC_USD_FEED);
    feeds.insert(USDC_ADDRESS, USDC_USD_FEED);
    feeds
}
