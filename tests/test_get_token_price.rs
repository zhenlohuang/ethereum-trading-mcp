//! Integration tests for the get_token_price tool.
//!
//! Run with: `cargo test --test test_get_token_price -- --ignored`

mod common;

use ethereum_trading_mcp::mcp::GetTokenPriceInput;
use rmcp::handler::server::wrapper::Parameters;

/// Test getting WETH price in USD.
#[tokio::test]
#[ignore = "Requires network access and environment variables"]
async fn test_get_weth_price_usd() {
    let server = skip_if_no_server!();

    let input =
        GetTokenPriceInput { token: "WETH".to_string(), quote_currency: Some("USD".to_string()) };

    let result = server.get_token_price(Parameters(input)).await;

    assert!(result.is_ok(), "get_token_price should succeed: {:?}", result.err());

    let json_str = result.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    // Verify response structure
    assert!(parsed.get("token").is_some());
    assert!(parsed.get("price").is_some());
    assert!(parsed.get("quote_currency").is_some());
    assert!(parsed.get("source").is_some());
    assert!(parsed.get("timestamp").is_some());

    // Verify price is a valid number and reasonable (ETH should be > $100)
    let price_str = parsed["price"].as_str().unwrap();
    let price: f64 = price_str.parse().unwrap();
    assert!(price > 100.0, "ETH price should be > $100, got {}", price);
    assert!(price < 100000.0, "ETH price should be < $100,000, got {}", price);

    println!("WETH Price (USD): {}", json_str);
}

/// Test getting WETH price in ETH (should be 1:1).
#[tokio::test]
#[ignore = "Requires network access and environment variables"]
async fn test_get_weth_price_eth() {
    let server = skip_if_no_server!();

    let input =
        GetTokenPriceInput { token: "WETH".to_string(), quote_currency: Some("ETH".to_string()) };

    let result = server.get_token_price(Parameters(input)).await;

    assert!(result.is_ok(), "get_token_price should succeed: {:?}", result.err());

    let json_str = result.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    // WETH to ETH should be exactly 1
    let price_str = parsed["price"].as_str().unwrap();
    assert_eq!(price_str, "1", "WETH/ETH price should be 1, got {}", price_str);

    println!("WETH Price (ETH): {}", json_str);
}

/// Test getting USDC price in USD (should be ~1).
#[tokio::test]
#[ignore = "Requires network access and environment variables"]
async fn test_get_usdc_price_usd() {
    let server = skip_if_no_server!();

    let input =
        GetTokenPriceInput { token: "USDC".to_string(), quote_currency: Some("USD".to_string()) };

    let result = server.get_token_price(Parameters(input)).await;

    assert!(result.is_ok(), "get_token_price should succeed: {:?}", result.err());

    let json_str = result.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    // USDC to USD should be approximately 1 (±0.02 for stablecoin)
    let price_str = parsed["price"].as_str().unwrap();
    let price: f64 = price_str.parse().unwrap();
    assert!(price > 0.98, "USDC price should be > 0.98, got {}", price);
    assert!(price < 1.02, "USDC price should be < 1.02, got {}", price);

    println!("USDC Price (USD): {}", json_str);
}

/// Test getting UNI token price.
#[tokio::test]
#[ignore = "Requires network access and environment variables"]
async fn test_get_uni_price() {
    let server = skip_if_no_server!();

    let input =
        GetTokenPriceInput { token: "UNI".to_string(), quote_currency: Some("USD".to_string()) };

    let result = server.get_token_price(Parameters(input)).await;

    assert!(result.is_ok(), "get_token_price should succeed: {:?}", result.err());

    let json_str = result.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    // UNI should have a price > 0
    let price_str = parsed["price"].as_str().unwrap();
    let price: f64 = price_str.parse().unwrap();
    assert!(price > 0.0, "UNI price should be > 0, got {}", price);

    println!("UNI Price (USD): {}", json_str);
}

/// Test default quote currency (should be USD).
#[tokio::test]
#[ignore = "Requires network access and environment variables"]
async fn test_get_price_default_quote() {
    let server = skip_if_no_server!();

    let input = GetTokenPriceInput { token: "WETH".to_string(), quote_currency: None };

    let result = server.get_token_price(Parameters(input)).await;

    assert!(result.is_ok(), "get_token_price should succeed: {:?}", result.err());

    let json_str = result.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    // Default quote currency should be USD
    assert_eq!(parsed["quote_currency"], "USD");

    println!("WETH Price (default): {}", json_str);
}

/// Test unknown token handling.
#[tokio::test]
#[ignore = "Requires network access and environment variables"]
async fn test_get_price_unknown_token() {
    let server = skip_if_no_server!();

    let input = GetTokenPriceInput {
        token: "NOTAREALTOKEN123".to_string(),
        quote_currency: Some("USD".to_string()),
    };

    let result = server.get_token_price(Parameters(input)).await;

    // Should return an error for unknown token
    assert!(result.is_err(), "get_token_price should fail for unknown token");
}

/// Test invalid quote currency handling.
#[tokio::test]
#[ignore = "Requires network access and environment variables"]
async fn test_get_price_invalid_quote() {
    let server = skip_if_no_server!();

    let input = GetTokenPriceInput {
        token: "WETH".to_string(),
        quote_currency: Some("INVALID".to_string()),
    };

    let result = server.get_token_price(Parameters(input)).await;

    // Should return an error for invalid quote currency
    assert!(result.is_err(), "get_token_price should fail for invalid quote currency");
}
