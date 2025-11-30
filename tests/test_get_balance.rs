//! Integration tests for the get_balance tool.
//!
//! Run with: `cargo test --test test_get_balance -- --ignored`

mod common;

use ethereum_trading_mcp::mcp::GetBalanceInput;
use rmcp::handler::server::wrapper::Parameters;

/// Test querying ETH balance for Vitalik's address.
#[tokio::test]
#[ignore = "Requires network access and environment variables"]
async fn test_get_eth_balance() {
    let server = skip_if_no_server!();

    // Vitalik's public address (well-known, always has ETH)
    let input = GetBalanceInput {
        address: "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045".to_string(),
        token_address: None,
    };

    let result = server.get_balance(Parameters(input)).await;

    assert!(result.is_ok(), "get_balance should succeed: {:?}", result.err());

    let json_str = result.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    // Verify response structure
    assert!(parsed.get("address").is_some());
    assert!(parsed.get("token").is_some());
    assert!(parsed.get("balance").is_some());
    assert!(parsed.get("balance_raw").is_some());

    // Verify token info for ETH
    let token = &parsed["token"];
    assert_eq!(token["symbol"], "ETH");
    assert_eq!(token["decimals"], 18);
    assert!(token["address"].is_null());

    println!("ETH Balance Result: {}", json_str);
}

/// Test querying USDC balance for a known address.
#[tokio::test]
#[ignore = "Requires network access and environment variables"]
async fn test_get_erc20_balance() {
    let server = skip_if_no_server!();

    // USDC contract address on mainnet
    let usdc_address = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";

    // Query a well-known address (Circle's treasury or any holder)
    let input = GetBalanceInput {
        address: "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045".to_string(),
        token_address: Some(usdc_address.to_string()),
    };

    let result = server.get_balance(Parameters(input)).await;

    assert!(result.is_ok(), "get_balance for ERC20 should succeed: {:?}", result.err());

    let json_str = result.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    // Verify response structure
    assert!(parsed.get("address").is_some());
    assert!(parsed.get("token").is_some());
    assert!(parsed.get("balance").is_some());
    assert!(parsed.get("balance_raw").is_some());

    // Verify token info for USDC
    let token = &parsed["token"];
    assert_eq!(token["symbol"], "USDC");
    assert_eq!(token["decimals"], 6);
    assert!(!token["address"].is_null());

    println!("USDC Balance Result: {}", json_str);
}

/// Test invalid address handling.
#[tokio::test]
#[ignore = "Requires network access and environment variables"]
async fn test_get_balance_invalid_address() {
    let server = skip_if_no_server!();

    let input = GetBalanceInput { address: "not-a-valid-address".to_string(), token_address: None };

    let result = server.get_balance(Parameters(input)).await;

    // Should return an error for invalid address
    assert!(result.is_err(), "get_balance should fail for invalid address");
}

/// Test empty address handling.
#[tokio::test]
#[ignore = "Requires network access and environment variables"]
async fn test_get_balance_empty_address() {
    let server = skip_if_no_server!();

    let input = GetBalanceInput { address: "".to_string(), token_address: None };

    let result = server.get_balance(Parameters(input)).await;

    assert!(result.is_err(), "get_balance should fail for empty address");
}
