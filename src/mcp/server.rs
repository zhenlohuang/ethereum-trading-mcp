//! MCP server implementation.

use std::sync::Arc;

use alloy::primitives::Address;
use rmcp::{
    handler::server::router::tool::ToolRouter,
    handler::server::wrapper::Parameters,
    model::{Implementation, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
};
use rust_decimal::Decimal;

use crate::{
    config::Config,
    error::AppError,
    ethereum::{EthereumClient, WalletManager},
    services::{BalanceService, PriceService, SwapService, TokenRegistry},
    types::{parse_units, QuoteCurrency, SwapParams},
};

/// Ethereum Trading MCP Server.
///
/// Provides tools for querying balances, prices, and simulating token swaps.
#[derive(Clone)]
pub struct EthereumTradingServer {
    balance_service: BalanceService,
    price_service: PriceService,
    swap_service: SwapService,
    token_registry: Arc<TokenRegistry>,
    tool_router: ToolRouter<Self>,
}

impl EthereumTradingServer {
    /// Create a new Ethereum Trading MCP Server.
    ///
    /// Note: This uses lazy initialization - no network calls are made during
    /// server startup. The Ethereum connection is established when the first
    /// tool is invoked.
    pub fn new(config: Config) -> Result<Self, AppError> {
        tracing::info!("Initializing Ethereum Trading MCP Server");

        // Initialize Ethereum client (lazy - no network call yet)
        let client = Arc::new(EthereumClient::new(&config.rpc_url)?);

        // Initialize wallet
        let wallet = WalletManager::from_private_key(&config.private_key)?;

        // Initialize token registry with chain ID from config
        let token_registry = Arc::new(TokenRegistry::new(config.chain_id));

        // Initialize services
        let balance_service = BalanceService::new(client.clone());
        let price_service = PriceService::new(client.clone(), balance_service.clone());
        let swap_service = SwapService::new(client, wallet, balance_service.clone());

        tracing::info!("Ethereum Trading MCP Server initialized successfully");

        Ok(Self {
            balance_service,
            price_service,
            swap_service,
            token_registry,
            tool_router: Self::tool_router(),
        })
    }
}

/// Input parameters for the get_balance tool.
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct GetBalanceInput {
    /// Wallet address to query (0x...).
    pub address: String,
    /// Optional ERC20 token contract address. If not provided, returns native ETH balance.
    #[serde(default)]
    pub token_address: Option<String>,
}

/// Input parameters for the get_token_price tool.
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct GetTokenPriceInput {
    /// Token symbol (e.g., "WETH", "USDC", "UNI").
    pub token: String,
    /// Quote currency: "USD" or "ETH". Defaults to "USD".
    #[serde(default)]
    pub quote_currency: Option<String>,
}

/// Input parameters for the swap_tokens tool.
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct SwapTokensInput {
    /// Input token symbol (e.g., "WETH", "USDC").
    pub from_token: String,
    /// Output token symbol (e.g., "WETH", "USDC").
    pub to_token: String,
    /// Amount to swap (human-readable, e.g., "1.5").
    pub amount: String,
    /// Slippage tolerance percentage (default: 0.5).
    #[serde(default)]
    pub slippage_tolerance: Option<f64>,
}

/// Parse an Ethereum address from a string.
fn parse_address(s: &str) -> Result<Address, McpError> {
    s.parse::<Address>()
        .map_err(|_| McpError::invalid_params(format!("Invalid address: {}", s), None))
}

#[tool_router]
impl EthereumTradingServer {
    /// Query ETH and ERC20 token balances for a wallet address.
    ///
    /// Returns the balance in both human-readable format (with proper decimals)
    /// and raw format (smallest unit like wei).
    #[tool(description = "Query ETH and ERC20 token balances for a wallet address")]
    pub async fn get_balance(
        &self,
        Parameters(input): Parameters<GetBalanceInput>,
    ) -> Result<String, McpError> {
        tracing::info!(
            address = %input.address,
            token = ?input.token_address,
            "get_balance called"
        );

        let address = parse_address(&input.address)?;
        let token_address = input.token_address.as_ref().map(|s| parse_address(s)).transpose()?;

        let result = self
            .balance_service
            .get_balance(address, token_address)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    /// Get current token price in USD or ETH.
    ///
    /// Fetches prices from on-chain sources (Chainlink oracles or Uniswap pools).
    /// Token symbols are resolved using Uniswap Token List.
    #[tool(
        description = "Get current token price in USD or ETH from on-chain sources. Supports any token from Uniswap Token List (e.g., WETH, USDC, UNI, LINK, etc.)."
    )]
    pub async fn get_token_price(
        &self,
        Parameters(input): Parameters<GetTokenPriceInput>,
    ) -> Result<String, McpError> {
        tracing::info!(
            token = %input.token,
            quote = ?input.quote_currency,
            "get_token_price called"
        );

        // Resolve token symbol using TokenRegistry
        let token_entry =
            self.token_registry.resolve_symbol(&input.token).await.ok_or_else(|| {
                McpError::invalid_params(
                    format!(
                        "Unknown token symbol: '{}'. Token not found in Uniswap Token List.",
                        input.token
                    ),
                    None,
                )
            })?;

        let quote_currency = input
            .quote_currency
            .as_ref()
            .map(|s| s.parse::<QuoteCurrency>().map_err(|e| McpError::invalid_params(e, None)))
            .transpose()?
            .unwrap_or_default();

        let result = self
            .price_service
            .get_price(token_entry.address, quote_currency)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    /// Simulate a token swap on Uniswap V2/V3.
    ///
    /// Constructs a real Uniswap transaction and simulates it using eth_call.
    /// The transaction is NOT executed on-chain.
    ///
    /// Returns estimated output amount, gas costs, price impact, and the raw transaction data.
    #[tool(
        description = "Simulate a token swap on Uniswap V2/V3 without executing on-chain. Supports any token from Uniswap Token List."
    )]
    pub async fn swap_tokens(
        &self,
        Parameters(input): Parameters<SwapTokensInput>,
    ) -> Result<String, McpError> {
        tracing::info!(
            from = %input.from_token,
            to = %input.to_token,
            amount = %input.amount,
            slippage = ?input.slippage_tolerance,
            "swap_tokens called"
        );

        // Resolve token symbols using TokenRegistry
        let from_entry =
            self.token_registry.resolve_symbol(&input.from_token).await.ok_or_else(|| {
                McpError::invalid_params(
                    format!(
                        "Unknown from_token symbol: '{}'. Token not found in Uniswap Token List.",
                        input.from_token
                    ),
                    None,
                )
            })?;

        let to_entry =
            self.token_registry.resolve_symbol(&input.to_token).await.ok_or_else(|| {
                McpError::invalid_params(
                    format!(
                        "Unknown to_token symbol: '{}'. Token not found in Uniswap Token List.",
                        input.to_token
                    ),
                    None,
                )
            })?;

        // Use decimals from TokenRegistry
        let amount_in = parse_units(&input.amount, from_entry.decimals)
            .map_err(|e| McpError::invalid_params(e, None))?;

        let slippage_tolerance = input
            .slippage_tolerance
            .map(|s| Decimal::try_from(s).unwrap_or(Decimal::new(5, 1)))
            .unwrap_or(Decimal::new(5, 1)); // Default 0.5%

        let params = SwapParams {
            from_token: from_entry.address,
            to_token: to_entry.address,
            amount_in,
            slippage_tolerance,
            deadline: None,
        };

        let result = self
            .swap_service
            .simulate_swap(params)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(e.to_string(), None))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for EthereumTradingServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "ethereum-trading-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                title: None,
                icons: None,
                website_url: None,
            },
            instructions: Some(
                "Ethereum Trading MCP Server. Provides tools for querying balances, \
                 token prices, and simulating Uniswap swaps."
                    .to_string(),
            ),
        }
    }
}
