//! Integration tests for the swap_tokens tool.
//!
//! Run with: `cargo test --test test_swap_tokens -- --ignored`

mod common;

use ethereum_trading_mcp::mcp::SwapTokensInput;
use rmcp::handler::server::wrapper::Parameters;

/// Test simulating WETH to USDC swap.
#[tokio::test]
#[ignore = "Requires network access and environment variables"]
async fn test_swap_weth_to_usdc() {
    let server = skip_if_no_server!();

    let input = SwapTokensInput {
        from_token: "WETH".to_string(),
        to_token: "USDC".to_string(),
        amount: "0.1".to_string(),
        slippage_tolerance: Some("0.5".to_string()),
    };

    let result = server.swap_tokens(Parameters(input)).await;

    assert!(result.is_ok(), "swap_tokens should succeed: {:?}", result.err());

    let json_str = result.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    // Verify response structure
    assert!(parsed.get("simulation_success").is_some());
    assert!(parsed.get("amount_in").is_some());
    assert!(parsed.get("amount_out_expected").is_some());
    assert!(parsed.get("amount_out_minimum").is_some());
    assert!(parsed.get("price_impact").is_some());
    assert!(parsed.get("gas_estimate").is_some());
    assert!(parsed.get("gas_price").is_some());
    assert!(parsed.get("gas_cost_eth").is_some());
    assert!(parsed.get("route").is_some());
    assert!(parsed.get("transaction").is_some());

    // Verify amount_in matches input
    assert_eq!(parsed["amount_in"], "0.1");

    // Verify route contains protocol and path
    let route = &parsed["route"];
    assert!(route.get("protocol").is_some());
    assert!(route.get("path").is_some());

    // Verify transaction has required fields
    let tx = &parsed["transaction"];
    assert!(tx.get("to").is_some());
    assert!(tx.get("data").is_some());

    println!("WETH->USDC Swap Result: {}", json_str);
}

/// Test simulating USDC to WETH swap.
#[tokio::test]
#[ignore = "Requires network access and environment variables"]
async fn test_swap_usdc_to_weth() {
    let server = skip_if_no_server!();

    let input = SwapTokensInput {
        from_token: "USDC".to_string(),
        to_token: "WETH".to_string(),
        amount: "100".to_string(), // 100 USDC
        slippage_tolerance: Some("1.0".to_string()),
    };

    let result = server.swap_tokens(Parameters(input)).await;

    assert!(result.is_ok(), "swap_tokens should succeed: {:?}", result.err());

    let json_str = result.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    // Verify response structure
    assert!(parsed.get("simulation_success").is_some());
    assert!(parsed.get("amount_out_expected").is_some());

    // Amount out should be a reasonable ETH amount (< 1 ETH for 100 USDC)
    let amount_out_str = parsed["amount_out_expected"].as_str().unwrap();
    let amount_out: f64 = amount_out_str.parse().unwrap();
    assert!(amount_out > 0.0, "Should get some WETH output");
    assert!(amount_out < 1.0, "100 USDC should be < 1 ETH");

    println!("USDC->WETH Swap Result: {}", json_str);
}

/// Test swap with default slippage tolerance.
#[tokio::test]
#[ignore = "Requires network access and environment variables"]
async fn test_swap_default_slippage() {
    let server = skip_if_no_server!();

    let input = SwapTokensInput {
        from_token: "WETH".to_string(),
        to_token: "USDC".to_string(),
        amount: "0.05".to_string(),
        slippage_tolerance: None, // Should default to 0.5%
    };

    let result = server.swap_tokens(Parameters(input)).await;

    assert!(result.is_ok(), "swap_tokens should succeed: {:?}", result.err());

    let json_str = result.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    // Verify minimum amount is ~99.5% of expected (0.5% slippage)
    let expected_str = parsed["amount_out_expected"].as_str().unwrap();
    let minimum_str = parsed["amount_out_minimum"].as_str().unwrap();
    let expected: f64 = expected_str.parse().unwrap();
    let minimum: f64 = minimum_str.parse().unwrap();

    let ratio = minimum / expected;
    assert!(ratio > 0.99 && ratio <= 1.0, "Default slippage should be 0.5%, ratio: {}", ratio);

    println!("Default Slippage Result: {}", json_str);
}

/// Test swap with UNI token.
#[tokio::test]
#[ignore = "Requires network access and environment variables"]
async fn test_swap_uni_to_weth() {
    let server = skip_if_no_server!();

    let input = SwapTokensInput {
        from_token: "UNI".to_string(),
        to_token: "WETH".to_string(),
        amount: "10".to_string(), // 10 UNI
        slippage_tolerance: Some("1.0".to_string()),
    };

    let result = server.swap_tokens(Parameters(input)).await;

    assert!(result.is_ok(), "swap_tokens should succeed: {:?}", result.err());

    let json_str = result.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    // UNI swap should work
    assert!(parsed.get("amount_out_expected").is_some());
    let amount_out_str = parsed["amount_out_expected"].as_str().unwrap();
    let amount_out: f64 = amount_out_str.parse().unwrap();
    assert!(amount_out > 0.0, "Should get some WETH output for UNI");

    println!("UNI->WETH Swap Result: {}", json_str);
}

/// Test swap with same from and to token (should fail).
#[tokio::test]
#[ignore = "Requires network access and environment variables"]
async fn test_swap_same_token_error() {
    let server = skip_if_no_server!();

    let input = SwapTokensInput {
        from_token: "WETH".to_string(),
        to_token: "WETH".to_string(),
        amount: "1".to_string(),
        slippage_tolerance: None,
    };

    let result = server.swap_tokens(Parameters(input)).await;

    assert!(result.is_err(), "swap_tokens should fail for same token");
}

/// Test swap with zero amount (should fail).
#[tokio::test]
#[ignore = "Requires network access and environment variables"]
async fn test_swap_zero_amount_error() {
    let server = skip_if_no_server!();

    let input = SwapTokensInput {
        from_token: "WETH".to_string(),
        to_token: "USDC".to_string(),
        amount: "0".to_string(),
        slippage_tolerance: None,
    };

    let result = server.swap_tokens(Parameters(input)).await;

    assert!(result.is_err(), "swap_tokens should fail for zero amount");
}

/// Test swap with invalid slippage tolerance (should fail).
#[tokio::test]
#[ignore = "Requires network access and environment variables"]
async fn test_swap_invalid_slippage_error() {
    let server = skip_if_no_server!();

    let input = SwapTokensInput {
        from_token: "WETH".to_string(),
        to_token: "USDC".to_string(),
        amount: "1".to_string(),
        slippage_tolerance: Some("100".to_string()), // 100% is too high
    };

    let result = server.swap_tokens(Parameters(input)).await;

    assert!(result.is_err(), "swap_tokens should fail for slippage > 50%");
}

/// Test swap with unknown token (should fail).
#[tokio::test]
#[ignore = "Requires network access and environment variables"]
async fn test_swap_unknown_token_error() {
    let server = skip_if_no_server!();

    let input = SwapTokensInput {
        from_token: "NOTAREALTOKEN".to_string(),
        to_token: "USDC".to_string(),
        amount: "1".to_string(),
        slippage_tolerance: None,
    };

    let result = server.swap_tokens(Parameters(input)).await;

    assert!(result.is_err(), "swap_tokens should fail for unknown token");
}
