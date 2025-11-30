//! Ethereum Trading MCP Server
//!
//! A Model Context Protocol server for Ethereum trading operations.

use rmcp::ServiceExt;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use ethereum_trading_mcp::{Config, EthereumTradingServer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = Config::from_env()?;

    // Initialize logging
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(filter)
        .init();

    tracing::info!("Starting Ethereum Trading MCP Server");

    // Create the server
    let server = EthereumTradingServer::new(config)?;

    // Run with stdio transport
    let transport = rmcp::transport::stdio();
    let running = server.serve(transport).await?;

    // Wait for the server to finish
    running.waiting().await?;

    Ok(())
}
