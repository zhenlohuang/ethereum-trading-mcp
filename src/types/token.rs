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
    // Handle zero case explicitly
    if value == U256::ZERO {
        return "0".to_string();
    }

    let value_str = value.to_string();
    let decimals = decimals as usize;

    if decimals == 0 {
        return value_str;
    }

    let len = value_str.len();
    if len <= decimals {
        // Value is less than 1, pad with zeros
        let zeros = decimals - len;
        let decimal_part = value_str.trim_end_matches('0');
        if decimal_part.is_empty() {
            "0".to_string()
        } else {
            format!("0.{}{}", "0".repeat(zeros), decimal_part)
        }
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
    let amount = amount.trim();

    // Check for empty input
    if amount.is_empty() {
        return Err("Amount cannot be empty".to_string());
    }

    // Check for negative numbers
    if amount.starts_with('-') {
        return Err("Amount cannot be negative".to_string());
    }

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
    use alloy::primitives::address;

    // ============================================================================
    // TokenInfo Tests
    // ============================================================================

    #[test]
    fn test_token_info_eth() {
        let info = TokenInfo::eth();
        assert_eq!(info.symbol, "ETH");
        assert_eq!(info.decimals, 18);
        assert!(info.address.is_none());
    }

    #[test]
    fn test_token_info_erc20() {
        let addr = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
        let info = TokenInfo::erc20(addr, "USDC".to_string(), 6);

        assert_eq!(info.symbol, "USDC");
        assert_eq!(info.decimals, 6);
        assert!(info.address.is_some());
        // Address is formatted in lowercase, so compare case-insensitively
        let addr_lower = info.address.as_ref().unwrap().to_lowercase();
        assert!(addr_lower.contains("a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"));
    }

    #[test]
    fn test_token_info_serialization() {
        let info = TokenInfo::eth();
        let json = serde_json::to_string(&info).unwrap();

        // ETH should not have address field (skip_serializing_if)
        assert!(!json.contains("address"));
        assert!(json.contains("\"symbol\":\"ETH\""));
        assert!(json.contains("\"decimals\":18"));
    }

    #[test]
    fn test_token_info_erc20_serialization() {
        let addr = address!("0000000000000000000000000000000000000001");
        let info = TokenInfo::erc20(addr, "TEST".to_string(), 8);
        let json = serde_json::to_string(&info).unwrap();

        assert!(json.contains("address"));
        assert!(json.contains("\"symbol\":\"TEST\""));
        assert!(json.contains("\"decimals\":8"));
    }

    #[test]
    fn test_token_info_deserialization() {
        let json = r#"{"symbol":"DAI","decimals":18}"#;
        let info: TokenInfo = serde_json::from_str(json).unwrap();

        assert_eq!(info.symbol, "DAI");
        assert_eq!(info.decimals, 18);
        assert!(info.address.is_none());
    }

    // ============================================================================
    // BalanceInfo Tests
    // ============================================================================

    #[test]
    fn test_balance_info_creation() {
        let info = BalanceInfo {
            address: "0x1234...".to_string(),
            token: TokenInfo::eth(),
            balance: "1.5".to_string(),
            balance_raw: "1500000000000000000".to_string(),
        };

        assert_eq!(info.balance, "1.5");
        assert_eq!(info.token.symbol, "ETH");
    }

    #[test]
    fn test_balance_info_serialization() {
        let info = BalanceInfo {
            address: "0xABC".to_string(),
            token: TokenInfo::eth(),
            balance: "10".to_string(),
            balance_raw: "10000000000000000000".to_string(),
        };

        let json = serde_json::to_string(&info).unwrap();
        let parsed: BalanceInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.address, info.address);
        assert_eq!(parsed.balance, info.balance);
    }

    // ============================================================================
    // QuoteCurrency Tests
    // ============================================================================

    #[test]
    fn test_quote_currency_default() {
        let default = QuoteCurrency::default();
        assert_eq!(default, QuoteCurrency::USD);
    }

    #[test]
    fn test_quote_currency_from_str() {
        assert_eq!("USD".parse::<QuoteCurrency>().unwrap(), QuoteCurrency::USD);
        assert_eq!("usd".parse::<QuoteCurrency>().unwrap(), QuoteCurrency::USD);
        assert_eq!("Usd".parse::<QuoteCurrency>().unwrap(), QuoteCurrency::USD);
        assert_eq!("ETH".parse::<QuoteCurrency>().unwrap(), QuoteCurrency::ETH);
        assert_eq!("eth".parse::<QuoteCurrency>().unwrap(), QuoteCurrency::ETH);
    }

    #[test]
    fn test_quote_currency_from_str_invalid() {
        assert!("BTC".parse::<QuoteCurrency>().is_err());
        assert!("USDT".parse::<QuoteCurrency>().is_err());
        assert!("".parse::<QuoteCurrency>().is_err());
    }

    #[test]
    fn test_quote_currency_serialization() {
        let usd = QuoteCurrency::USD;
        let json = serde_json::to_string(&usd).unwrap();
        assert_eq!(json, "\"USD\"");

        let eth = QuoteCurrency::ETH;
        let json = serde_json::to_string(&eth).unwrap();
        assert_eq!(json, "\"ETH\"");
    }

    #[test]
    fn test_quote_currency_deserialization() {
        let usd: QuoteCurrency = serde_json::from_str("\"USD\"").unwrap();
        assert_eq!(usd, QuoteCurrency::USD);

        let eth: QuoteCurrency = serde_json::from_str("\"ETH\"").unwrap();
        assert_eq!(eth, QuoteCurrency::ETH);
    }

    // ============================================================================
    // PriceSource Tests
    // ============================================================================

    #[test]
    fn test_price_source_serialization() {
        assert_eq!(serde_json::to_string(&PriceSource::Chainlink).unwrap(), "\"chainlink\"");
        assert_eq!(serde_json::to_string(&PriceSource::UniswapV2).unwrap(), "\"uniswap_v2\"");
        assert_eq!(serde_json::to_string(&PriceSource::UniswapV3).unwrap(), "\"uniswap_v3\"");
    }

    #[test]
    fn test_price_source_deserialization() {
        let chainlink: PriceSource = serde_json::from_str("\"chainlink\"").unwrap();
        assert_eq!(chainlink, PriceSource::Chainlink);

        let v2: PriceSource = serde_json::from_str("\"uniswap_v2\"").unwrap();
        assert_eq!(v2, PriceSource::UniswapV2);
    }

    // ============================================================================
    // PriceInfo Tests
    // ============================================================================

    #[test]
    fn test_price_info_creation() {
        let info = PriceInfo {
            token: TokenInfo::eth(),
            price: "3000.50".to_string(),
            quote_currency: QuoteCurrency::USD,
            source: PriceSource::Chainlink,
            timestamp: 1700000000,
        };

        assert_eq!(info.price, "3000.50");
        assert_eq!(info.quote_currency, QuoteCurrency::USD);
        assert_eq!(info.source, PriceSource::Chainlink);
    }

    #[test]
    fn test_price_info_serialization() {
        let info = PriceInfo {
            token: TokenInfo::eth(),
            price: "2500".to_string(),
            quote_currency: QuoteCurrency::USD,
            source: PriceSource::UniswapV3,
            timestamp: 1234567890,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"price\":\"2500\""));
        assert!(json.contains("\"quote_currency\":\"USD\""));
        assert!(json.contains("\"source\":\"uniswap_v3\""));
    }

    // ============================================================================
    // format_units Tests
    // ============================================================================

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
    fn test_format_units_zero() {
        assert_eq!(format_units(U256::ZERO, 18), "0");
        assert_eq!(format_units(U256::ZERO, 6), "0");
        assert_eq!(format_units(U256::ZERO, 0), "0");
    }

    #[test]
    fn test_format_units_no_decimals() {
        let value = U256::from(12345u64);
        assert_eq!(format_units(value, 0), "12345");
    }

    #[test]
    fn test_format_units_small_values() {
        // 1 wei
        let one_wei = U256::from(1u64);
        assert_eq!(format_units(one_wei, 18), "0.000000000000000001");

        // 100 wei
        let hundred_wei = U256::from(100u64);
        assert_eq!(format_units(hundred_wei, 18), "0.0000000000000001");
    }

    #[test]
    fn test_format_units_trailing_zeros_removed() {
        // 1.5 ETH
        let value = U256::from(1_500_000_000_000_000_000u64);
        assert_eq!(format_units(value, 18), "1.5");

        // 10.00 USDC should be "10", not "10.00"
        let value = U256::from(10_000_000u64);
        assert_eq!(format_units(value, 6), "10");
    }

    #[test]
    fn test_format_units_large_values() {
        // 1 million ETH
        let million_eth = U256::from(1_000_000u64) * U256::from(10u64).pow(U256::from(18u64));
        assert_eq!(format_units(million_eth, 18), "1000000");
    }

    #[test]
    fn test_format_units_precision() {
        // 1.123456789012345678 ETH
        let value = U256::from(1_123_456_789_012_345_678u64);
        assert_eq!(format_units(value, 18), "1.123456789012345678");
    }

    // ============================================================================
    // parse_units Tests
    // ============================================================================

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

    #[test]
    fn test_parse_units_negative() {
        let result = parse_units("-1", 18);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Amount cannot be negative");

        let result = parse_units("-0.5", 18);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_units_empty() {
        let result = parse_units("", 18);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Amount cannot be empty");

        let result = parse_units("   ", 18);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_units_whitespace_trimming() {
        let result = parse_units("  1.5  ", 18).unwrap();
        assert_eq!(result, U256::from(1_500_000_000_000_000_000u64));
    }

    #[test]
    fn test_parse_units_decimal_only() {
        // ".5" should be parsed as 0.5
        let result = parse_units(".5", 18).unwrap();
        assert_eq!(result, U256::from(500_000_000_000_000_000u64));
    }

    #[test]
    fn test_parse_units_excess_decimals_truncated() {
        // More decimals than token supports should be truncated
        let result = parse_units("1.1234567", 6).unwrap();
        assert_eq!(result, U256::from(1_123_456u64)); // Truncated to 6 decimals
    }

    #[test]
    fn test_parse_units_fewer_decimals_padded() {
        // Fewer decimals should be padded
        let result = parse_units("1.5", 6).unwrap();
        assert_eq!(result, U256::from(1_500_000u64));
    }

    #[test]
    fn test_parse_units_zero_decimals() {
        let result = parse_units("100", 0).unwrap();
        assert_eq!(result, U256::from(100u64));
    }

    #[test]
    fn test_parse_units_invalid_format() {
        // Multiple decimal points
        let result = parse_units("1.2.3", 18);
        assert!(result.is_err());

        // Invalid characters
        let result = parse_units("1.5abc", 18);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_units_large_values() {
        let result = parse_units("1000000", 18).unwrap();
        let expected = U256::from(1_000_000u64) * U256::from(10u64).pow(U256::from(18u64));
        assert_eq!(result, expected);
    }

    // ============================================================================
    // u256_to_decimal Tests
    // ============================================================================

    #[test]
    fn test_u256_to_decimal_whole_number() {
        let value = U256::from(1_000_000_000_000_000_000u64);
        let decimal = u256_to_decimal(value, 18);
        assert_eq!(decimal, Decimal::from(1));
    }

    #[test]
    fn test_u256_to_decimal_fractional() {
        let value = U256::from(1_500_000_000_000_000_000u64);
        let decimal = u256_to_decimal(value, 18);
        assert_eq!(decimal, Decimal::new(15, 1)); // 1.5
    }

    #[test]
    fn test_u256_to_decimal_zero() {
        let value = U256::ZERO;
        let decimal = u256_to_decimal(value, 18);
        assert_eq!(decimal, Decimal::ZERO);
    }

    #[test]
    fn test_u256_to_decimal_small_value() {
        let value = U256::from(1u64); // 1 wei
        let decimal = u256_to_decimal(value, 18);
        // Very small number, should be close to 0
        assert!(decimal < Decimal::new(1, 10)); // Less than 0.0000000001
    }

    // ============================================================================
    // Round-trip Tests
    // ============================================================================

    #[test]
    fn test_format_parse_roundtrip() {
        let original = U256::from(1_234_567_890_123_456_789u64);
        let formatted = format_units(original, 18);
        let parsed = parse_units(&formatted, 18).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn test_format_parse_roundtrip_usdc() {
        let original = U256::from(1_234_567u64);
        let formatted = format_units(original, 6);
        let parsed = parse_units(&formatted, 6).unwrap();
        assert_eq!(original, parsed);
    }
}
