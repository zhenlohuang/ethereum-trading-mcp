//! Swap-related types.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Parameters for a swap operation.
#[derive(Debug, Clone)]
pub struct SwapParams {
    /// Input token address.
    pub from_token: alloy::primitives::Address,
    /// Output token address.
    pub to_token: alloy::primitives::Address,
    /// Amount to swap in smallest units.
    pub amount_in: alloy::primitives::U256,
    /// Slippage tolerance as a percentage (e.g., 0.5 for 0.5%).
    pub slippage_tolerance: Decimal,
    /// Transaction deadline (Unix timestamp).
    pub deadline: Option<u64>,
}

/// Uniswap protocol version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UniswapVersion {
    /// Uniswap V2.
    V2,
    /// Uniswap V3.
    V3,
}

/// Swap route information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapRoute {
    /// Protocol version used.
    pub protocol: UniswapVersion,
    /// Token path for the swap.
    pub path: Vec<String>,
    /// Fee tier (only for V3, in basis points).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_tier: Option<u32>,
}

/// Raw transaction data for inspection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionData {
    /// Target contract address.
    pub to: String,
    /// Calldata (hex encoded).
    pub data: String,
    /// Value in wei (hex encoded).
    pub value: String,
}

/// Result of a swap simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapSimulationResult {
    /// Whether the simulation was successful (transaction would execute).
    pub simulation_success: bool,
    /// Error message if simulation failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub simulation_error: Option<String>,
    /// Input amount (human-readable).
    pub amount_in: String,
    /// Expected output amount (human-readable).
    pub amount_out_expected: String,
    /// Minimum output after slippage (human-readable).
    pub amount_out_minimum: String,
    /// Price impact as a percentage.
    pub price_impact: String,
    /// Estimated gas units.
    pub gas_estimate: String,
    /// Current gas price in wei.
    pub gas_price: String,
    /// Gas cost in ETH (human-readable).
    pub gas_cost_eth: String,
    /// Swap route used.
    pub route: SwapRoute,
    /// Raw transaction data.
    pub transaction: TransactionData,
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::{Address, U256};

    #[test]
    fn test_uniswap_version_serialization() {
        // Test V2 serialization
        let v2 = UniswapVersion::V2;
        let serialized = serde_json::to_string(&v2).unwrap();
        assert_eq!(serialized, "\"v2\"");

        // Test V3 serialization
        let v3 = UniswapVersion::V3;
        let serialized = serde_json::to_string(&v3).unwrap();
        assert_eq!(serialized, "\"v3\"");
    }

    #[test]
    fn test_uniswap_version_deserialization() {
        let v2: UniswapVersion = serde_json::from_str("\"v2\"").unwrap();
        assert_eq!(v2, UniswapVersion::V2);

        let v3: UniswapVersion = serde_json::from_str("\"v3\"").unwrap();
        assert_eq!(v3, UniswapVersion::V3);
    }

    #[test]
    fn test_uniswap_version_equality() {
        assert_eq!(UniswapVersion::V2, UniswapVersion::V2);
        assert_eq!(UniswapVersion::V3, UniswapVersion::V3);
        assert_ne!(UniswapVersion::V2, UniswapVersion::V3);
    }

    #[test]
    fn test_swap_route_v2_creation() {
        let route = SwapRoute {
            protocol: UniswapVersion::V2,
            path: vec!["0xToken1".to_string(), "0xToken2".to_string()],
            fee_tier: None,
        };

        assert_eq!(route.protocol, UniswapVersion::V2);
        assert_eq!(route.path.len(), 2);
        assert!(route.fee_tier.is_none());
    }

    #[test]
    fn test_swap_route_v3_creation() {
        let route = SwapRoute {
            protocol: UniswapVersion::V3,
            path: vec!["0xWETH".to_string(), "0xUSDC".to_string()],
            fee_tier: Some(3000), // 0.3%
        };

        assert_eq!(route.protocol, UniswapVersion::V3);
        assert_eq!(route.fee_tier, Some(3000));
    }

    #[test]
    fn test_swap_route_multihop() {
        let route = SwapRoute {
            protocol: UniswapVersion::V2,
            path: vec!["0xToken1".to_string(), "0xWETH".to_string(), "0xToken2".to_string()],
            fee_tier: None,
        };

        assert_eq!(route.path.len(), 3);
    }

    #[test]
    fn test_swap_route_serialization() {
        let route = SwapRoute {
            protocol: UniswapVersion::V3,
            path: vec!["0xA".to_string(), "0xB".to_string()],
            fee_tier: Some(500),
        };

        let json = serde_json::to_string(&route).unwrap();
        assert!(json.contains("\"protocol\":\"v3\""));
        assert!(json.contains("\"fee_tier\":500"));

        // Deserialize and verify
        let parsed: SwapRoute = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.protocol, route.protocol);
        assert_eq!(parsed.fee_tier, route.fee_tier);
    }

    #[test]
    fn test_swap_route_fee_tier_skip_serializing_if_none() {
        let route = SwapRoute {
            protocol: UniswapVersion::V2,
            path: vec!["0xA".to_string(), "0xB".to_string()],
            fee_tier: None,
        };

        let json = serde_json::to_string(&route).unwrap();
        // fee_tier should be omitted when None
        assert!(!json.contains("fee_tier"));
    }

    #[test]
    fn test_transaction_data_creation() {
        let tx = TransactionData {
            to: "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D".to_string(),
            data: "0x38ed1739".to_string(),
            value: "0".to_string(),
        };

        assert!(!tx.to.is_empty());
        assert!(tx.data.starts_with("0x"));
    }

    #[test]
    fn test_transaction_data_serialization() {
        let tx = TransactionData {
            to: "0xRouter".to_string(),
            data: "0xcalldata".to_string(),
            value: "1000000000000000000".to_string(),
        };

        let json = serde_json::to_string(&tx).unwrap();
        let parsed: TransactionData = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.to, tx.to);
        assert_eq!(parsed.data, tx.data);
        assert_eq!(parsed.value, tx.value);
    }

    #[test]
    fn test_swap_params_creation() {
        let params = SwapParams {
            from_token: Address::ZERO,
            to_token: Address::ZERO,
            amount_in: U256::from(1_000_000u64),
            slippage_tolerance: Decimal::new(5, 1), // 0.5%
            deadline: Some(1700000000),
        };

        assert_eq!(params.slippage_tolerance, Decimal::new(5, 1));
        assert_eq!(params.deadline, Some(1700000000));
    }

    #[test]
    fn test_swap_params_without_deadline() {
        let params = SwapParams {
            from_token: Address::ZERO,
            to_token: Address::ZERO,
            amount_in: U256::from(100u64),
            slippage_tolerance: Decimal::ONE,
            deadline: None,
        };

        assert!(params.deadline.is_none());
    }

    #[test]
    fn test_swap_simulation_result_success() {
        let result = SwapSimulationResult {
            simulation_success: true,
            simulation_error: None,
            amount_in: "1.0".to_string(),
            amount_out_expected: "3000.0".to_string(),
            amount_out_minimum: "2985.0".to_string(),
            price_impact: "0.05".to_string(),
            gas_estimate: "150000".to_string(),
            gas_price: "30000000000".to_string(),
            gas_cost_eth: "0.0045".to_string(),
            route: SwapRoute {
                protocol: UniswapVersion::V3,
                path: vec!["WETH".to_string(), "USDC".to_string()],
                fee_tier: Some(3000),
            },
            transaction: TransactionData {
                to: "0xRouter".to_string(),
                data: "0x".to_string(),
                value: "0".to_string(),
            },
        };

        assert!(result.simulation_success);
        assert!(result.simulation_error.is_none());
    }

    #[test]
    fn test_swap_simulation_result_failure() {
        let result = SwapSimulationResult {
            simulation_success: false,
            simulation_error: Some("Insufficient liquidity".to_string()),
            amount_in: "1000.0".to_string(),
            amount_out_expected: "0".to_string(),
            amount_out_minimum: "0".to_string(),
            price_impact: "0".to_string(),
            gas_estimate: "200000".to_string(),
            gas_price: "30000000000".to_string(),
            gas_cost_eth: "0.006".to_string(),
            route: SwapRoute {
                protocol: UniswapVersion::V2,
                path: vec!["TokenA".to_string(), "TokenB".to_string()],
                fee_tier: None,
            },
            transaction: TransactionData {
                to: "0x".to_string(),
                data: "0x".to_string(),
                value: "0".to_string(),
            },
        };

        assert!(!result.simulation_success);
        assert!(result.simulation_error.is_some());
        assert_eq!(result.simulation_error.unwrap(), "Insufficient liquidity");
    }

    #[test]
    fn test_swap_simulation_result_serialization() {
        let result = SwapSimulationResult {
            simulation_success: true,
            simulation_error: None,
            amount_in: "1.0".to_string(),
            amount_out_expected: "100.0".to_string(),
            amount_out_minimum: "99.5".to_string(),
            price_impact: "0.01".to_string(),
            gas_estimate: "100000".to_string(),
            gas_price: "20000000000".to_string(),
            gas_cost_eth: "0.002".to_string(),
            route: SwapRoute {
                protocol: UniswapVersion::V3,
                path: vec!["A".to_string(), "B".to_string()],
                fee_tier: Some(500),
            },
            transaction: TransactionData {
                to: "0xRouter".to_string(),
                data: "0xdata".to_string(),
                value: "0".to_string(),
            },
        };

        let json = serde_json::to_string(&result).unwrap();

        // simulation_error should be omitted when None
        assert!(!json.contains("simulation_error"));

        // Other fields should be present
        assert!(json.contains("simulation_success"));
        assert!(json.contains("amount_in"));
        assert!(json.contains("route"));
    }
}
