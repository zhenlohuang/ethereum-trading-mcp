//! Token-related types.

use alloy::primitives::{Address, U256};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Information about a token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    /// Token contract address (None for native ETH).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    /// Token symbol (e.g., "ETH", "USDC").
    pub symbol: String,
    /// Number of decimals.
    pub decimals: u8,
}

impl TokenInfo {
    /// Create a new TokenInfo for native ETH.
    pub fn eth() -> Self {
        Self { address: None, symbol: "ETH".to_string(), decimals: 18 }
    }

    /// Create a new TokenInfo for an ERC20 token.
    pub fn erc20(address: Address, symbol: String, decimals: u8) -> Self {
        Self { address: Some(format!("{address:?}")), symbol, decimals }
    }
}

/// Balance information response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceInfo {
    /// Wallet address.
    pub address: String,
    /// Token information.
    pub token: TokenInfo,
    /// Human-readable balance with proper decimals.
    pub balance: String,
    /// Raw balance in smallest unit.
    pub balance_raw: String,
}

/// Quote currency for price queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum QuoteCurrency {
    /// US Dollar.
    #[default]
    USD,
    /// Ether.
    ETH,
}

impl std::str::FromStr for QuoteCurrency {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "USD" => Ok(QuoteCurrency::USD),
            "ETH" => Ok(QuoteCurrency::ETH),
            _ => Err(format!("Invalid quote currency: {}", s)),
        }
    }
}

/// Source of price data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PriceSource {
    /// Chainlink oracle.
    Chainlink,
    /// Uniswap V2 pool.
    UniswapV2,
    /// Uniswap V3 pool.
    UniswapV3,
}

/// Price information response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceInfo {
    /// Token information.
    pub token: TokenInfo,
    /// Current price.
    pub price: String,
    /// Quote currency.
    pub quote_currency: QuoteCurrency,
    /// Price data source.
    pub source: PriceSource,
    /// Timestamp of price data.
    pub timestamp: u64,
}

/// Format a U256 value with decimals to a human-readable string.
pub fn format_units(value: U256, decimals: u8) -> String {
    let value_str = value.to_string();
    let decimals = decimals as usize;

    if decimals == 0 {
        return value_str;
    }

    let len = value_str.len();
    if len <= decimals {
        // Value is less than 1, pad with zeros
        let zeros = decimals - len;
        format!("0.{}{}", "0".repeat(zeros), value_str.trim_end_matches('0'))
    } else {
        // Split into integer and decimal parts
        let (integer, decimal) = value_str.split_at(len - decimals);
        let decimal = decimal.trim_end_matches('0');
        if decimal.is_empty() {
            integer.to_string()
        } else {
            format!("{}.{}", integer, decimal)
        }
    }
}

/// Parse a human-readable amount string to U256 with decimals.
pub fn parse_units(amount: &str, decimals: u8) -> Result<U256, String> {
    let decimals = decimals as usize;
    let parts: Vec<&str> = amount.split('.').collect();

    match parts.len() {
        1 => {
            // No decimal point
            let value = parts[0].parse::<U256>().map_err(|e| format!("Invalid amount: {}", e))?;
            let multiplier = U256::from(10).pow(U256::from(decimals));
            Ok(value * multiplier)
        }
        2 => {
            let integer = parts[0];
            let mut fraction = parts[1].to_string();

            // Pad or truncate fraction to match decimals
            if fraction.len() > decimals {
                fraction.truncate(decimals);
            } else {
                fraction.push_str(&"0".repeat(decimals - fraction.len()));
            }

            let integer_value = if integer.is_empty() {
                U256::ZERO
            } else {
                integer.parse::<U256>().map_err(|e| format!("Invalid integer part: {}", e))?
            };

            let fraction_value = if fraction.is_empty() {
                U256::ZERO
            } else {
                fraction.parse::<U256>().map_err(|e| format!("Invalid fraction part: {}", e))?
            };

            let multiplier = U256::from(10).pow(U256::from(decimals));
            Ok(integer_value * multiplier + fraction_value)
        }
        _ => Err("Invalid amount format".to_string()),
    }
}

/// Convert U256 to Decimal with proper scaling.
pub fn u256_to_decimal(value: U256, decimals: u8) -> Decimal {
    let formatted = format_units(value, decimals);
    formatted.parse::<Decimal>().unwrap_or(Decimal::ZERO)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_units() {
        // 1 ETH = 10^18 wei
        let one_eth = U256::from(1_000_000_000_000_000_000u64);
        assert_eq!(format_units(one_eth, 18), "1");

        // 0.5 ETH
        let half_eth = U256::from(500_000_000_000_000_000u64);
        assert_eq!(format_units(half_eth, 18), "0.5");

        // 1 USDC = 10^6 units
        let one_usdc = U256::from(1_000_000u64);
        assert_eq!(format_units(one_usdc, 6), "1");
    }

    #[test]
    fn test_parse_units() {
        // 1 ETH
        let result = parse_units("1", 18).unwrap();
        assert_eq!(result, U256::from(1_000_000_000_000_000_000u64));

        // 0.5 ETH
        let result = parse_units("0.5", 18).unwrap();
        assert_eq!(result, U256::from(500_000_000_000_000_000u64));

        // 100 USDC
        let result = parse_units("100", 6).unwrap();
        assert_eq!(result, U256::from(100_000_000u64));
    }
}
