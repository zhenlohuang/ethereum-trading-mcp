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
