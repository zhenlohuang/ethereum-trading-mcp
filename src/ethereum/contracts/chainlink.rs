//! Chainlink price feed contract bindings.

use alloy::sol;
use std::collections::HashMap;

use super::{USDC_ADDRESS, WBTC_ADDRESS, WETH_ADDRESS};

// Re-export Chainlink feed addresses from constants module.
pub use crate::ethereum::constants::{BTC_USD_FEED, ETH_USD_FEED, USDC_USD_FEED};

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
