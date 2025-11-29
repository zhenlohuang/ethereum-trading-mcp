# Ethereum Trading MCP Server - System Design

## 1. Overview

This document describes the system design for an Ethereum Trading MCP (Model Context Protocol) Server built in Rust. The server enables AI agents to query blockchain data and simulate token swaps on Ethereum through a standardized protocol.

### 1.1 Goals

- Provide a reliable MCP server for Ethereum trading operations
- Enable real-time balance queries for ETH and ERC20 tokens
- Support token price lookups from on-chain sources
- Simulate Uniswap V2/V3 swaps without actual on-chain execution
- Maintain high code quality with proper error handling and logging

### 1.2 Non-Goals

- Actual on-chain transaction execution (only simulation)
- Support for other DEXs besides Uniswap
- Multi-chain support (Ethereum mainnet only in MVP)
- Historical data analysis

## 2. Architecture

### 2.1 High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              AI Agent (Client)                               │
└─────────────────────────────────────────────────────────────────────────────┘
                                       │
                                       │ MCP Protocol (JSON-RPC 2.0 over stdio)
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                              MCP Server Layer                                │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                         Tool Router                                   │   │
│  │   ┌─────────────┐   ┌─────────────────┐   ┌─────────────────────┐   │   │
│  │   │ get_balance │   │ get_token_price │   │    swap_tokens      │   │   │
│  │   └─────────────┘   └─────────────────┘   └─────────────────────┘   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                             Service Layer                                    │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────────────┐  │
│  │  Balance Service │  │   Price Service  │  │      Swap Service        │  │
│  └──────────────────┘  └──────────────────┘  └──────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                            Ethereum Layer                                    │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────────────┐  │
│  │  Ethereum Client │  │ Contract Bindings│  │    Wallet Manager        │  │
│  │    (alloy)       │  │  (ERC20/Uniswap) │  │                          │  │
│  └──────────────────┘  └──────────────────┘  └──────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
                                       │
                                       │ JSON-RPC
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Ethereum RPC Provider                                │
│                    (Infura / Alchemy / Public Node)                          │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 Component Overview

| Component | Responsibility |
|-----------|---------------|
| MCP Server | Handle JSON-RPC 2.0 requests, route to tools |
| Tool Router | Dispatch tool calls to appropriate handlers |
| Balance Service | Query ETH/ERC20 balances |
| Price Service | Fetch token prices from on-chain sources |
| Swap Service | Construct and simulate Uniswap swaps |
| Ethereum Client | Manage RPC connections and calls |
| Contract Bindings | ABI definitions for ERC20, Uniswap contracts |
| Wallet Manager | Handle private key and transaction signing |

## 3. Module Structure

```
src/
├── main.rs                 # Application entry point
├── lib.rs                  # Library root, re-exports
├── config/
│   └── mod.rs              # Configuration management
├── error/
│   └── mod.rs              # Error types and handling
├── mcp/
│   ├── mod.rs              # MCP module root
│   ├── server.rs           # MCP server implementation
│   └── tools/
│       ├── mod.rs          # Tool definitions
│       ├── get_balance.rs  # Balance query tool
│       ├── get_token_price.rs  # Price query tool
│       └── swap_tokens.rs  # Swap simulation tool
├── ethereum/
│   ├── mod.rs              # Ethereum module root
│   ├── client.rs           # Ethereum RPC client
│   ├── wallet.rs           # Wallet management
│   └── contracts/
│       ├── mod.rs          # Contract module root
│       ├── erc20.rs        # ERC20 ABI and helpers
│       ├── uniswap_v2.rs   # Uniswap V2 contracts
│       └── uniswap_v3.rs   # Uniswap V3 contracts
├── services/
│   ├── mod.rs              # Services module root
│   ├── balance.rs          # Balance query logic
│   ├── price.rs            # Price fetching logic
│   └── swap.rs             # Swap simulation logic
└── types/
    ├── mod.rs              # Types module root
    ├── token.rs            # Token-related types
    └── swap.rs             # Swap-related types
```

## 4. Detailed Component Design

### 4.1 MCP Server

The MCP server handles communication with AI agents using the Model Context Protocol over stdio.

#### 4.1.1 Server Initialization

```rust
pub struct EthereumTradingServer {
    ethereum_client: Arc<EthereumClient>,
    balance_service: BalanceService,
    price_service: PriceService,
    swap_service: SwapService,
}

impl EthereumTradingServer {
    pub async fn new(config: Config) -> Result<Self> {
        let ethereum_client = Arc::new(EthereumClient::new(&config.rpc_url).await?);
        // Initialize services...
    }
}
```

#### 4.1.2 Tool Registration

Using the `rmcp` crate, tools are registered with the `#[tool]` macro:

```rust
#[tool(description = "Query ETH and ERC20 token balances for a wallet address")]
async fn get_balance(
    &self,
    #[arg(description = "Wallet address to query")] address: String,
    #[arg(description = "Optional ERC20 token contract address")] token_address: Option<String>,
) -> Result<BalanceResponse, McpError> {
    // Implementation
}
```

### 4.2 Ethereum Client

The Ethereum client wraps the `alloy` library for blockchain interactions.

#### 4.2.1 Client Structure

```rust
pub struct EthereumClient {
    provider: Arc<RootProvider<Http<Client>>>,
    chain_id: u64,
}

impl EthereumClient {
    pub async fn new(rpc_url: &str) -> Result<Self> {
        let provider = ProviderBuilder::new()
            .on_http(rpc_url.parse()?);
        let chain_id = provider.get_chain_id().await?;
        Ok(Self { provider: Arc::new(provider), chain_id })
    }

    pub async fn get_eth_balance(&self, address: Address) -> Result<U256> {
        self.provider.get_balance(address).await
    }

    pub async fn call(&self, tx: TransactionRequest) -> Result<Bytes> {
        self.provider.call(&tx).await
    }
}
```

### 4.3 Balance Service

Handles balance queries for both native ETH and ERC20 tokens.

#### 4.3.1 Service Interface

```rust
pub struct BalanceService {
    client: Arc<EthereumClient>,
}

impl BalanceService {
    pub async fn get_balance(
        &self,
        address: Address,
        token_address: Option<Address>,
    ) -> Result<BalanceInfo> {
        match token_address {
            None => self.get_eth_balance(address).await,
            Some(token) => self.get_erc20_balance(address, token).await,
        }
    }
}
```

#### 4.3.2 Response Types

```rust
pub struct BalanceInfo {
    pub address: Address,
    pub token: TokenInfo,
    pub balance: String,      // Human-readable with decimals
    pub balance_raw: U256,    // Raw wei/smallest unit
}

pub struct TokenInfo {
    pub address: Option<Address>,  // None for ETH
    pub symbol: String,
    pub decimals: u8,
}
```

### 4.4 Price Service

Fetches token prices from on-chain sources (Uniswap pools, Chainlink oracles).

#### 4.4.1 Price Sources

1. **Uniswap V3 TWAP**: Time-weighted average price from pool observations
2. **Uniswap V2 Reserves**: Spot price from pool reserves
3. **Chainlink Oracles**: For major tokens with price feeds

#### 4.4.2 Service Interface

```rust
pub struct PriceService {
    client: Arc<EthereumClient>,
    chainlink_feeds: HashMap<Address, Address>,  // token -> feed
}

impl PriceService {
    pub async fn get_price(
        &self,
        token_address: Address,
        quote_currency: QuoteCurrency,
    ) -> Result<PriceInfo> {
        // Try Chainlink first, fall back to Uniswap
    }
}

pub enum QuoteCurrency {
    USD,
    ETH,
}

pub struct PriceInfo {
    pub token: TokenInfo,
    pub price: Decimal,
    pub quote_currency: QuoteCurrency,
    pub source: PriceSource,
    pub timestamp: u64,
}
```

### 4.5 Swap Service

Constructs and simulates Uniswap swaps without on-chain execution.

#### 4.5.1 Swap Flow

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   Validate   │────▶│  Find Route  │────▶│  Build Tx    │────▶│  Simulate    │
│   Inputs     │     │  (V2/V3)     │     │              │     │  (eth_call)  │
└──────────────┘     └──────────────┘     └──────────────┘     └──────────────┘
```

#### 4.5.2 Service Interface

```rust
pub struct SwapService {
    client: Arc<EthereumClient>,
    wallet: WalletManager,
    router_v2: Address,
    router_v3: Address,
}

impl SwapService {
    pub async fn simulate_swap(
        &self,
        params: SwapParams,
    ) -> Result<SwapSimulationResult> {
        // 1. Validate tokens and amounts
        // 2. Find best route (V2 vs V3)
        // 3. Build swap transaction
        // 4. Simulate via eth_call
        // 5. Return estimated output and gas
    }
}

pub struct SwapParams {
    pub from_token: Address,
    pub to_token: Address,
    pub amount_in: U256,
    pub slippage_tolerance: Decimal,  // e.g., 0.5%
    pub deadline: Option<u64>,
}

pub struct SwapSimulationResult {
    pub amount_in: String,
    pub amount_out_expected: String,
    pub amount_out_minimum: String,  // After slippage
    pub price_impact: Decimal,
    pub gas_estimate: U256,
    pub gas_price: U256,
    pub gas_cost_eth: String,
    pub route: SwapRoute,
    pub transaction: TransactionData,  // Raw tx for inspection
}
```

#### 4.5.3 Uniswap V2 Swap

```rust
// Router address: 0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D
// Function: swapExactTokensForTokens(
//     uint amountIn,
//     uint amountOutMin,
//     address[] calldata path,
//     address to,
//     uint deadline
// )

pub async fn build_v2_swap_tx(&self, params: &SwapParams) -> Result<TransactionRequest> {
    let path = vec![params.from_token, params.to_token];
    let deadline = params.deadline.unwrap_or_else(|| current_timestamp() + 1200);

    let calldata = uniswap_v2_router::swapExactTokensForTokensCall {
        amountIn: params.amount_in,
        amountOutMin: calculate_min_out(params),
        path,
        to: self.wallet.address(),
        deadline: U256::from(deadline),
    }.abi_encode();

    Ok(TransactionRequest::default()
        .to(self.router_v2)
        .input(calldata.into()))
}
```

#### 4.5.4 Uniswap V3 Swap

```rust
// SwapRouter address: 0xE592427A0AEce92De3Edee1F18E0157C05861564
// Function: exactInputSingle(ExactInputSingleParams calldata params)

pub async fn build_v3_swap_tx(&self, params: &SwapParams) -> Result<TransactionRequest> {
    let swap_params = ExactInputSingleParams {
        tokenIn: params.from_token,
        tokenOut: params.to_token,
        fee: find_best_fee_tier(params.from_token, params.to_token).await?,
        recipient: self.wallet.address(),
        deadline: U256::from(params.deadline.unwrap_or_else(|| current_timestamp() + 1200)),
        amountIn: params.amount_in,
        amountOutMinimum: calculate_min_out(params),
        sqrtPriceLimitX96: U256::ZERO,  // No price limit
    };

    let calldata = swap_router::exactInputSingleCall { params: swap_params }.abi_encode();

    Ok(TransactionRequest::default()
        .to(self.router_v3)
        .input(calldata.into()))
}
```

### 4.6 Wallet Manager

Handles private key management and transaction signing.

#### 4.6.1 Design

```rust
pub struct WalletManager {
    signer: LocalSigner<SigningKey>,
    address: Address,
}

impl WalletManager {
    pub fn from_env() -> Result<Self> {
        let private_key = std::env::var("ETHEREUM_PRIVATE_KEY")?;
        let signer = private_key.parse::<LocalSigner<SigningKey>>()?;
        let address = signer.address();
        Ok(Self { signer, address })
    }

    pub fn address(&self) -> Address {
        self.address
    }

    pub async fn sign_transaction(&self, tx: TransactionRequest) -> Result<TxEnvelope> {
        // Sign transaction (used if we need to submit)
    }
}
```

## 5. Configuration

### 5.1 Environment Variables

| Variable | Description | Required | Default |
|----------|-------------|----------|---------|
| `ETHEREUM_RPC_URL` | Ethereum JSON-RPC endpoint | Yes | - |
| `ETHEREUM_PRIVATE_KEY` | Private key for wallet (hex) | Yes | - |
| `ETHEREUM_CHAIN_ID` | Chain ID | No | 1 (mainnet) |
| `LOG_LEVEL` | Logging level | No | `info` |

### 5.2 Config Structure

```rust
pub struct Config {
    pub rpc_url: String,
    pub chain_id: u64,
    pub private_key: String,
    pub log_level: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            rpc_url: env::var("ETHEREUM_RPC_URL")?,
            chain_id: env::var("ETHEREUM_CHAIN_ID")
                .unwrap_or_else(|_| "1".to_string())
                .parse()?,
            private_key: env::var("ETHEREUM_PRIVATE_KEY")?,
            log_level: env::var("LOG_LEVEL")
                .unwrap_or_else(|_| "info".to_string()),
        })
    }
}
```

## 6. Error Handling

### 6.1 Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Ethereum RPC error: {0}")]
    Rpc(#[from] alloy::transports::TransportError),

    #[error("Invalid address: {0}")]
    InvalidAddress(String),

    #[error("Token not found: {0}")]
    TokenNotFound(Address),

    #[error("Insufficient liquidity for swap")]
    InsufficientLiquidity,

    #[error("Slippage too high: expected {expected}, got {actual}")]
    SlippageExceeded { expected: String, actual: String },

    #[error("Wallet error: {0}")]
    Wallet(String),

    #[error("Simulation failed: {0}")]
    SimulationFailed(String),
}
```

### 6.2 Error Mapping to MCP

```rust
impl From<AppError> for McpError {
    fn from(err: AppError) -> Self {
        match err {
            AppError::InvalidAddress(_) => McpError::invalid_params(err.to_string()),
            AppError::TokenNotFound(_) => McpError::invalid_params(err.to_string()),
            _ => McpError::internal_error(err.to_string()),
        }
    }
}
```

## 7. Data Flow Examples

### 7.1 Get Balance Flow

```
AI Agent                    MCP Server                  Ethereum Node
    │                           │                            │
    │  tools/call               │                            │
    │  get_balance              │                            │
    │  {address, token?}        │                            │
    │ ─────────────────────────▶│                            │
    │                           │                            │
    │                           │  eth_call (balanceOf)      │
    │                           │ ──────────────────────────▶│
    │                           │                            │
    │                           │  balance (U256)            │
    │                           │ ◀──────────────────────────│
    │                           │                            │
    │                           │  eth_call (decimals)       │
    │                           │ ──────────────────────────▶│
    │                           │                            │
    │                           │  decimals (u8)             │
    │                           │ ◀──────────────────────────│
    │                           │                            │
    │  {balance, decimals,      │                            │
    │   symbol, formatted}      │                            │
    │ ◀─────────────────────────│                            │
```

### 7.2 Swap Simulation Flow

```
AI Agent                    MCP Server                  Ethereum Node
    │                           │                            │
    │  tools/call               │                            │
    │  swap_tokens              │                            │
    │  {from, to, amount, ...}  │                            │
    │ ─────────────────────────▶│                            │
    │                           │                            │
    │                           │  Get pool reserves/prices  │
    │                           │ ──────────────────────────▶│
    │                           │ ◀──────────────────────────│
    │                           │                            │
    │                           │  Build swap transaction    │
    │                           │  (construct calldata)      │
    │                           │                            │
    │                           │  eth_call (simulate swap)  │
    │                           │ ──────────────────────────▶│
    │                           │                            │
    │                           │  simulation result         │
    │                           │ ◀──────────────────────────│
    │                           │                            │
    │                           │  eth_estimateGas           │
    │                           │ ──────────────────────────▶│
    │                           │ ◀──────────────────────────│
    │                           │                            │
    │  {amountOut, gasEstimate, │                            │
    │   priceImpact, route}     │                            │
    │ ◀─────────────────────────│                            │
```

## 8. Contract Addresses (Ethereum Mainnet)

### 8.1 Uniswap Contracts

| Contract | Address |
|----------|---------|
| Uniswap V2 Router | `0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D` |
| Uniswap V2 Factory | `0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f` |
| Uniswap V3 SwapRouter | `0xE592427A0AEce92De3Edee1F18E0157C05861564` |
| Uniswap V3 Factory | `0x1F98431c8aD98523631AE4a59f267346ea31F984` |
| Uniswap V3 Quoter V2 | `0x61fFE014bA17989E743c5F6cB21bF9697530B21e` |

### 8.2 Common Token Addresses

| Token | Address |
|-------|---------|
| WETH | `0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2` |
| USDC | `0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48` |
| USDT | `0xdAC17F958D2ee523a2206206994597C13D831ec7` |
| DAI | `0x6B175474E89094C44Da98b954EescdeCB5Bad14` |
| WBTC | `0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599` |

### 8.3 Chainlink Price Feeds

| Feed | Address |
|------|---------|
| ETH/USD | `0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419` |
| BTC/USD | `0xF4030086522a5bEEa4988F8cA5B36dbC97BeE88c` |
| USDC/USD | `0x8fFfFfd4AfB6115b954Bd326cbe7B4BA576818f6` |

## 9. Testing Strategy

### 9.1 Unit Tests

- Test individual service methods with mocked Ethereum client
- Test input validation and error handling
- Test decimal/formatting utilities

### 9.2 Integration Tests

- Test against forked mainnet (using Anvil)
- Verify balance queries return correct data
- Verify swap simulations produce valid transactions

### 9.3 Example Test Cases

```rust
#[tokio::test]
async fn test_get_eth_balance() {
    let client = EthereumClient::new("http://localhost:8545").await.unwrap();
    let balance = client.get_eth_balance(VITALIK_ADDRESS).await.unwrap();
    assert!(balance > U256::ZERO);
}

#[tokio::test]
async fn test_swap_simulation() {
    let service = SwapService::new(...).await.unwrap();
    let result = service.simulate_swap(SwapParams {
        from_token: WETH,
        to_token: USDC,
        amount_in: parse_ether("1").unwrap(),
        slippage_tolerance: Decimal::new(5, 1),  // 0.5%
        deadline: None,
    }).await.unwrap();

    assert!(result.amount_out_expected.parse::<f64>().unwrap() > 0.0);
    assert!(result.gas_estimate > U256::ZERO);
}
```

## 10. Security Considerations

### 10.1 Private Key Handling

- Private key loaded from environment variable only
- Never logged or exposed in error messages
- Consider supporting hardware wallets in future

### 10.2 Input Validation

- Validate all addresses are valid checksummed addresses
- Validate amounts are positive and within reasonable bounds
- Sanitize token symbols to prevent injection

### 10.3 RPC Security

- Use HTTPS for RPC connections
- Consider rate limiting for public endpoints
- Validate RPC responses match expected schemas

## 11. Future Enhancements

1. **Multi-chain Support**: Add support for L2s (Arbitrum, Optimism, Base)
2. **More DEXs**: Integrate Sushiswap, Curve, 1inch aggregator
3. **Advanced Routing**: Multi-hop swaps for better prices
4. **Gas Optimization**: EIP-1559 support, gas price estimation
5. **Caching**: Cache token metadata and recent prices
6. **WebSocket Support**: Real-time price updates via subscriptions

## 12. Dependencies

```toml
[dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }

# Ethereum
alloy = { version = "1.0", features = ["full"] }

# MCP Protocol
rmcp = { version = "0.9", features = ["server"] }

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Utilities
thiserror = "2"
rust_decimal = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
dotenvy = "0.15"
```

## 13. API Reference

### 13.1 get_balance

Query ETH or ERC20 token balance for a wallet address.

**Parameters:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `address` | string | Yes | Wallet address (0x...) |
| `token_address` | string | No | ERC20 token contract address |

**Response:**
```json
{
  "address": "0x...",
  "token": {
    "address": "0x..." | null,
    "symbol": "ETH",
    "decimals": 18
  },
  "balance": "1.234567890123456789",
  "balance_raw": "1234567890123456789"
}
```

### 13.2 get_token_price

Get current token price in USD or ETH.

**Parameters:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `token_address` | string | Yes | Token contract address |
| `quote_currency` | string | No | "USD" or "ETH" (default: "USD") |

**Response:**
```json
{
  "token": {
    "address": "0x...",
    "symbol": "WETH",
    "decimals": 18
  },
  "price": "2500.50",
  "quote_currency": "USD",
  "source": "chainlink",
  "timestamp": 1699999999
}
```

### 13.3 swap_tokens

Simulate a token swap on Uniswap.

**Parameters:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `from_token` | string | Yes | Input token address |
| `to_token` | string | Yes | Output token address |
| `amount` | string | Yes | Amount to swap (human-readable) |
| `slippage_tolerance` | number | No | Slippage % (default: 0.5) |

**Response:**
```json
{
  "amount_in": "1.0",
  "amount_out_expected": "2500.123456",
  "amount_out_minimum": "2487.622789",
  "price_impact": "0.05",
  "gas_estimate": "150000",
  "gas_price": "30000000000",
  "gas_cost_eth": "0.0045",
  "route": {
    "protocol": "uniswap_v3",
    "path": ["0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"],
    "fee_tier": 3000
  },
  "transaction": {
    "to": "0xE592427A0AEce92De3Edee1F18E0157C05861564",
    "data": "0x...",
    "value": "0"
  }
}
```

## 14. MCP Protocol Integration

### 14.1 Server Capabilities

The MCP server advertises the following capabilities during initialization:

```json
{
  "name": "ethereum-trading-mcp",
  "version": "0.1.0",
  "capabilities": {
    "tools": {}
  }
}
```

### 14.2 Tool Listing

When a client requests the tool list via `tools/list`, the server responds with:

```json
{
  "tools": [
    {
      "name": "get_balance",
      "description": "Query ETH and ERC20 token balances for a wallet address",
      "inputSchema": {
        "type": "object",
        "properties": {
          "address": {
            "type": "string",
            "description": "Wallet address to query (0x...)"
          },
          "token_address": {
            "type": "string",
            "description": "Optional ERC20 token contract address"
          }
        },
        "required": ["address"]
      }
    },
    {
      "name": "get_token_price",
      "description": "Get current token price in USD or ETH",
      "inputSchema": {
        "type": "object",
        "properties": {
          "token_address": {
            "type": "string",
            "description": "Token contract address"
          },
          "quote_currency": {
            "type": "string",
            "enum": ["USD", "ETH"],
            "description": "Quote currency for price"
          }
        },
        "required": ["token_address"]
      }
    },
    {
      "name": "swap_tokens",
      "description": "Simulate a token swap on Uniswap V2/V3",
      "inputSchema": {
        "type": "object",
        "properties": {
          "from_token": {
            "type": "string",
            "description": "Input token address"
          },
          "to_token": {
            "type": "string",
            "description": "Output token address"
          },
          "amount": {
            "type": "string",
            "description": "Amount to swap (human-readable)"
          },
          "slippage_tolerance": {
            "type": "number",
            "description": "Slippage tolerance percentage (default: 0.5)"
          }
        },
        "required": ["from_token", "to_token", "amount"]
      }
    }
  ]
}
```

### 14.3 Communication Transport

The MCP server uses **stdio transport** for communication:

```rust
use rmcp::transport::stdio;
use rmcp::ServiceExt;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // Create server instance
    let server = EthereumTradingServer::new(Config::from_env()?).await?;

    // Run server with stdio transport
    let service = server.serve();
    let transport = stdio::server();

    service.run(transport).await?;

    Ok(())
}
```

## 15. Logging and Observability

### 15.1 Structured Logging

All operations are logged using the `tracing` crate with structured fields:

```rust
#[instrument(skip(self), fields(address = %address, token = ?token_address))]
pub async fn get_balance(
    &self,
    address: Address,
    token_address: Option<Address>,
) -> Result<BalanceInfo> {
    tracing::info!("Querying balance");

    let result = self.balance_service.get_balance(address, token_address).await;

    match &result {
        Ok(info) => tracing::info!(balance = %info.balance, "Balance query successful"),
        Err(e) => tracing::error!(error = %e, "Balance query failed"),
    }

    result
}
```

### 15.2 Log Levels

| Level | Usage |
|-------|-------|
| ERROR | Unrecoverable errors, RPC failures |
| WARN | Recoverable issues, fallback paths |
| INFO | Request handling, key operations |
| DEBUG | Detailed operation flow |
| TRACE | Raw RPC calls and responses |

### 15.3 Key Metrics to Log

- Request duration
- RPC call count and latency
- Error rates by type
- Gas estimation accuracy

## 16. Deployment

### 16.1 Build

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release
```

### 16.2 Running the Server

```bash
# Set required environment variables
export ETHEREUM_RPC_URL="https://eth-mainnet.g.alchemy.com/v2/YOUR_API_KEY"
export ETHEREUM_PRIVATE_KEY="0x..."
export LOG_LEVEL="info"

# Run the server
./target/release/ethereum-trading-mcp
```

### 16.3 Claude Desktop Integration

Add to `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "ethereum-trading": {
      "command": "/path/to/ethereum-trading-mcp",
      "env": {
        "ETHEREUM_RPC_URL": "https://eth-mainnet.g.alchemy.com/v2/YOUR_API_KEY",
        "ETHEREUM_PRIVATE_KEY": "0x...",
        "LOG_LEVEL": "info"
      }
    }
  }
}
```

## 17. Glossary

| Term | Definition |
|------|------------|
| MCP | Model Context Protocol - A protocol for AI agents to interact with external tools |
| ERC20 | Ethereum token standard defining a common interface for fungible tokens |
| Uniswap V2 | Automated market maker DEX using constant product formula (x*y=k) |
| Uniswap V3 | Advanced AMM with concentrated liquidity positions |
| WETH | Wrapped Ether - ERC20 representation of native ETH |
| Slippage | Price movement between transaction submission and execution |
| eth_call | RPC method to simulate a transaction without broadcasting |
| Gas | Unit measuring computational effort on Ethereum |

## 18. References

1. [Model Context Protocol Specification](https://modelcontextprotocol.io/docs)
2. [Rust MCP SDK (rmcp)](https://github.com/modelcontextprotocol/rust-sdk)
3. [Alloy - Ethereum Library for Rust](https://github.com/alloy-rs/alloy)
4. [Uniswap V2 Documentation](https://docs.uniswap.org/contracts/v2/overview)
5. [Uniswap V3 Documentation](https://docs.uniswap.org/contracts/v3/overview)
6. [Ethereum JSON-RPC Specification](https://ethereum.org/en/developers/docs/apis/json-rpc/)
7. [Chainlink Price Feeds](https://docs.chain.link/data-feeds/price-feeds)
