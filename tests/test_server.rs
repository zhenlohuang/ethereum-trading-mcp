//! Integration tests for server initialization.
//!
//! Run with: `cargo test --test test_server`

mod common;

use rmcp::model::ServerInfo;
use rmcp::ServerHandler;

/// Test server info.
#[test]
fn test_server_info() {
    let server = skip_if_no_server!();
    let info: ServerInfo = server.get_info();

    assert_eq!(info.server_info.name, "ethereum-trading-mcp");
    assert!(!info.server_info.version.is_empty());
}
