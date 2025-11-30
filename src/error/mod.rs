//! Error types and handling module.
//!
//! Defines all application-specific error types and conversions.

use alloy::primitives::Address;
use rmcp::ErrorData as McpError;
use thiserror::Error;

/// Application-wide error type.
#[derive(Debug, Error)]
pub enum AppError {
    /// Configuration-related errors.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Ethereum RPC errors.
    #[error("Ethereum RPC error: {0}")]
    Rpc(String),

    /// Transport errors.
    #[error("Transport error: {0}")]
    Transport(String),

    /// Invalid Ethereum address.
    #[error("Invalid address: {0}")]
    InvalidAddress(String),

    /// Token not found or invalid.
    #[error("Token not found: {0}")]
    TokenNotFound(Address),

    /// Insufficient liquidity for swap.
    #[error("Insufficient liquidity for swap")]
    InsufficientLiquidity,

    /// Slippage exceeded.
    #[error("Slippage too high: expected {expected}, got {actual}")]
    SlippageExceeded { expected: String, actual: String },

    /// Wallet-related errors.
    #[error("Wallet error: {0}")]
    Wallet(String),

    /// Simulation failed.
    #[error("Simulation failed: {0}")]
    SimulationFailed(String),

    /// Pool not found.
    #[error("Pool not found for token pair")]
    PoolNotFound,

    /// Parse error.
    #[error("Parse error: {0}")]
    Parse(String),

    /// Price oracle failure (e.g., stale or invalid data).
    #[error("Price oracle error: {0}")]
    PriceOracle(String),

    /// Numeric overflow during conversion.
    #[error("Numeric overflow: {0}")]
    NumericOverflow(String),

    /// Pending transaction error.
    #[error("Pending transaction error: {0}")]
    PendingTransaction(String),
}

impl From<alloy::transports::TransportError> for AppError {
    fn from(err: alloy::transports::TransportError) -> Self {
        AppError::Transport(err.to_string())
    }
}

impl From<alloy::contract::Error> for AppError {
    fn from(err: alloy::contract::Error) -> Self {
        AppError::Rpc(err.to_string())
    }
}

impl From<alloy::signers::local::LocalSignerError> for AppError {
    fn from(err: alloy::signers::local::LocalSignerError) -> Self {
        AppError::Wallet(err.to_string())
    }
}

impl From<alloy::hex::FromHexError> for AppError {
    fn from(err: alloy::hex::FromHexError) -> Self {
        AppError::Parse(err.to_string())
    }
}

impl From<std::num::ParseIntError> for AppError {
    fn from(err: std::num::ParseIntError) -> Self {
        AppError::Parse(err.to_string())
    }
}

impl From<AppError> for McpError {
    fn from(err: AppError) -> Self {
        match err {
            AppError::InvalidAddress(_)
            | AppError::TokenNotFound(_)
            | AppError::Parse(_)
            | AppError::NumericOverflow(_) => McpError::invalid_params(err.to_string(), None),
            AppError::Config(_) => McpError::invalid_request(err.to_string(), None),
            _ => McpError::internal_error(err.to_string(), None),
        }
    }
}

/// Result type alias using AppError.
pub type Result<T> = std::result::Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::address;
    use rmcp::model::ErrorCode;

    #[test]
    fn test_app_error_config_display() {
        let err = AppError::Config("Missing RPC URL".to_string());
        assert_eq!(err.to_string(), "Configuration error: Missing RPC URL");
    }

    #[test]
    fn test_app_error_rpc_display() {
        let err = AppError::Rpc("Connection timeout".to_string());
        assert_eq!(err.to_string(), "Ethereum RPC error: Connection timeout");
    }

    #[test]
    fn test_app_error_transport_display() {
        let err = AppError::Transport("Network unreachable".to_string());
        assert_eq!(err.to_string(), "Transport error: Network unreachable");
    }

    #[test]
    fn test_app_error_invalid_address_display() {
        let err = AppError::InvalidAddress("0xinvalid".to_string());
        assert_eq!(err.to_string(), "Invalid address: 0xinvalid");
    }

    #[test]
    fn test_app_error_token_not_found_display() {
        let addr = address!("0000000000000000000000000000000000000001");
        let err = AppError::TokenNotFound(addr);
        assert!(err.to_string().contains("Token not found"));
    }

    #[test]
    fn test_app_error_insufficient_liquidity_display() {
        let err = AppError::InsufficientLiquidity;
        assert_eq!(err.to_string(), "Insufficient liquidity for swap");
    }

    #[test]
    fn test_app_error_slippage_exceeded_display() {
        let err =
            AppError::SlippageExceeded { expected: "100".to_string(), actual: "95".to_string() };
        assert!(err.to_string().contains("expected 100"));
        assert!(err.to_string().contains("got 95"));
    }

    #[test]
    fn test_app_error_wallet_display() {
        let err = AppError::Wallet("Invalid private key".to_string());
        assert_eq!(err.to_string(), "Wallet error: Invalid private key");
    }

    #[test]
    fn test_app_error_simulation_failed_display() {
        let err = AppError::SimulationFailed("Out of gas".to_string());
        assert_eq!(err.to_string(), "Simulation failed: Out of gas");
    }

    #[test]
    fn test_app_error_pool_not_found_display() {
        let err = AppError::PoolNotFound;
        assert_eq!(err.to_string(), "Pool not found for token pair");
    }

    #[test]
    fn test_app_error_parse_display() {
        let err = AppError::Parse("Invalid hex".to_string());
        assert_eq!(err.to_string(), "Parse error: Invalid hex");
    }

    #[test]
    fn test_app_error_price_oracle_display() {
        let err = AppError::PriceOracle("Stale data".to_string());
        assert_eq!(err.to_string(), "Price oracle error: Stale data");
    }

    #[test]
    fn test_app_error_numeric_overflow_display() {
        let err = AppError::NumericOverflow("Value too large".to_string());
        assert_eq!(err.to_string(), "Numeric overflow: Value too large");
    }

    #[test]
    fn test_app_error_pending_transaction_display() {
        let err = AppError::PendingTransaction("Tx stuck".to_string());
        assert_eq!(err.to_string(), "Pending transaction error: Tx stuck");
    }

    #[test]
    fn test_app_error_to_mcp_error_invalid_params() {
        // InvalidAddress should map to invalid_params
        let err = AppError::InvalidAddress("bad address".to_string());
        let mcp_err: McpError = err.into();
        assert_eq!(mcp_err.code, ErrorCode::INVALID_PARAMS);

        // TokenNotFound should map to invalid_params
        let addr = address!("0000000000000000000000000000000000000001");
        let err = AppError::TokenNotFound(addr);
        let mcp_err: McpError = err.into();
        assert_eq!(mcp_err.code, ErrorCode::INVALID_PARAMS);

        // Parse error should map to invalid_params
        let err = AppError::Parse("parse failed".to_string());
        let mcp_err: McpError = err.into();
        assert_eq!(mcp_err.code, ErrorCode::INVALID_PARAMS);

        // NumericOverflow should map to invalid_params
        let err = AppError::NumericOverflow("overflow".to_string());
        let mcp_err: McpError = err.into();
        assert_eq!(mcp_err.code, ErrorCode::INVALID_PARAMS);
    }

    #[test]
    fn test_app_error_to_mcp_error_invalid_request() {
        // Config errors should map to invalid_request
        let err = AppError::Config("config error".to_string());
        let mcp_err: McpError = err.into();
        assert_eq!(mcp_err.code, ErrorCode::INVALID_REQUEST);
    }

    #[test]
    fn test_app_error_to_mcp_error_internal_error() {
        // RPC errors should map to internal_error
        let err = AppError::Rpc("rpc failed".to_string());
        let mcp_err: McpError = err.into();
        assert_eq!(mcp_err.code, ErrorCode::INTERNAL_ERROR);

        // Transport errors should map to internal_error
        let err = AppError::Transport("transport failed".to_string());
        let mcp_err: McpError = err.into();
        assert_eq!(mcp_err.code, ErrorCode::INTERNAL_ERROR);

        // PoolNotFound should map to internal_error
        let err = AppError::PoolNotFound;
        let mcp_err: McpError = err.into();
        assert_eq!(mcp_err.code, ErrorCode::INTERNAL_ERROR);
    }

    #[test]
    fn test_app_error_debug_trait() {
        let err = AppError::Config("test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("Config"));
    }

    #[test]
    fn test_from_parse_int_error() {
        let parse_result: std::result::Result<i32, _> = "not_a_number".parse();
        let parse_err = parse_result.unwrap_err();
        let app_err: AppError = parse_err.into();

        match app_err {
            AppError::Parse(msg) => assert!(msg.contains("invalid")),
            _ => panic!("Expected Parse error"),
        }
    }

    #[test]
    fn test_mcp_error_message_preserved() {
        let err = AppError::Rpc("Connection refused".to_string());
        let mcp_err: McpError = err.into();
        assert!(mcp_err.message.contains("Connection refused"));
    }

    #[test]
    fn test_mcp_error_data_is_none() {
        let err = AppError::PoolNotFound;
        let mcp_err: McpError = err.into();
        assert!(mcp_err.data.is_none());
    }
}
