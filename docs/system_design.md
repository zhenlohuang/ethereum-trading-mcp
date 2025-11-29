# Ethereum Trading MCP Server – High-Level Design

Repository: `ethereum-trading-mcp`
Language: Rust (async, tokio)

## 1. Goals & Scope

Build an MCP server that exposes Ethereum trading capabilities as MCP tools:

* `get_balance` – query ETH and ERC20 balances from real Ethereum RPC.
* `get_token_price` – fetch current token prices (USD / ETH).
* `swap_tokens` – construct Uniswap V2/V3 swap transactions and **simulate** them on mainnet using `eth_call` (no real execution).

Non-goals (for now):

* No multi-chain routing.
* No complex routing across multiple DEXes.
* No persistent database (only in-memory runtime state).

---

## 2. High-Level Architecture

### 2.1 Architectural Style

* **Layered, service-oriented** architecture:

  * **MCP layer** – exposes tools, validates user input, converts to domain types.
  * **Domain/service layer** – balance, pricing, and swap services.
  * **Ethereum integration layer** – RPC client, Uniswap contracts, wallet/signing.
* Single binary crate to keep it simple, with clear internal modules that can later be split into workspace crates.

### 2.2 Main Components

1. **MCP Server (`mcp` module)**

   * Uses `rmcp` SDK (or manual JSON-RPC 2.0) to:

     * Register tools.
     * Handle requests.
     * Serialize responses and errors.
   * Delegates to service layer.

2. **Services (`services` module)**

   * `BalanceService`
   * `PriceService`
   * `SwapService`
   * Encapsulate business logic, orchestration, and error mapping.

3. **Ethereum Client (`eth` module)**

   * Thin abstraction over Alloy RPC client:

     * `get_balance`, `get_erc20_balance`
     * `call`, `estimate_gas`, `get_block`, etc.
   * Manages provider URL, chain id, timeouts, and retries.

4. **Uniswap Integration (`dex` module)**

   * `uniswap_v2` and `uniswap_v3` submodules.
   * Helpers for:

     * Contract addresses & ABIs.
     * Function encoding (router calls).
     * Quote/simulation helpers.

5. **Wallet & Signing (`wallet` module)**

   * Loads private key from env/config.
   * Derives `from` address.
   * Provides EIP-1559 transaction builder + signing utilities (for future execution).
   * For simulation, constructs transaction call data and uses `eth_call`.

6. **Config & Environment (`config` module)**

   * Central `AppConfig` struct (RPC URL, chain id, default slippage, Uniswap addresses, etc.).
   * Loads from environment variables and optional config file.

7. **Shared Types & Errors (`types`, `errors` modules)**

   * Domain DTOs for MCP requests/responses.
   * Unified error enum with internal vs. user-facing representations.

8. **Observability (`telemetry` module)**

   * `tracing` setup (subscriber, filters).
   * Contextual span fields: tool_name, wallet, chain, etc.

---

## 3. Module Layout

Proposed project structure:

```text
ethereum-trading-mcp/
  ├─ src/
  │  ├─ main.rs
  │  ├─ mcp/
  │  │  ├─ mod.rs
  │  │  ├─ server.rs
  │  │  └─ tools.rs
  │  ├─ services/
  │  │  ├─ mod.rs
  │  │  ├─ balance_service.rs
  │  │  ├─ price_service.rs
  │  │  └─ swap_service.rs
  │  ├─ eth/
  │  │  ├─ mod.rs
  │  │  ├─ client.rs
  │  │  └─ types.rs
  │  ├─ dex/
  │  │  ├─ mod.rs
  │  │  ├─ uniswap_v2.rs
  │  │  └─ uniswap_v3.rs
  │  ├─ wallet.rs
  │  ├─ config.rs
  │  ├─ types.rs
  │  ├─ errors.rs
  │  └─ telemetry.rs
  ├─ tests/
  │  ├─ integration_balance.rs
  │  ├─ integration_price.rs
  │  └─ integration_swap.rs
  ├─ README.md
  └─ docs/
     └─ design.md
```

This structure makes it straightforward to:

* Add new DEXes under `dex/`.
* Add new MCP tools in `mcp/tools.rs`.
* Add chain-specific clients if we want multi-chain later.

---

## 4. Core Data Models

### 4.1 MCP Request/Response Types

```rust
// src/types.rs

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use alloy_primitives::Address;

#[derive(Debug, Deserialize)]
pub struct GetBalanceInput {
    pub wallet_address: String,
    pub token_address: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BalanceInfo {
    pub wallet_address: String,
    pub token_address: Option<String>,
    pub symbol: String,
    pub decimals: u8,
    pub raw_balance: String, // hex or decimal string for full precision
    pub normalized_balance: Decimal,
    pub network: String,
}

#[derive(Debug, Deserialize)]
pub struct GetTokenPriceInput {
    pub token: String, // address or symbol
    pub quote_currency: PriceQuoteCurrency,
}

#[derive(Debug, Serialize)]
pub struct TokenPrice {
    pub base_symbol: String,
    pub base_address: Option<String>,
    pub quote_currency: PriceQuoteCurrency,
    pub price: Decimal,
    pub source: String, // e.g. "UniswapV2", "Mock", "External API"
}

#[derive(Debug, Deserialize)]
pub enum PriceQuoteCurrency {
    USD,
    ETH,
}

#[derive(Debug, Deserialize)]
pub struct SwapTokensInput {
    pub from_token: String, // address or symbol
    pub to_token: String,   // address or symbol
    pub amount: Decimal,    // human-readable
    pub slippage_bps: u32,  // basis points
    pub dex: Option<DexKind>,
}

#[derive(Debug, Deserialize)]
pub enum DexKind {
    UniswapV2,
    UniswapV3,
}

#[derive(Debug, Serialize)]
pub struct SwapSimulationResult {
    pub from_token: String,
    pub to_token: String,
    pub amount_in: Decimal,
    pub estimated_amount_out: Decimal,
    pub min_amount_out: Decimal,
    pub gas_estimate: u64,
    pub gas_price_gwei: Decimal,
    pub estimated_fee_eth: Decimal,
    pub dex: DexKind,
    pub route: Vec<String>, // token addresses
}
```

### 4.2 Internal Domain Types

* `TokenId` – enum for symbol vs address.
* `TokenMetadata` – symbol, decimals, address.
* `ChainId`, `Network` – simple identifiers, maybe from Alloy types.
* `TxBuildContext` – gas, nonce, chain id, from address.

---

## 5. MCP Layer Design

### 5.1 MCP Server Startup

`main.rs`:

1. Load `AppConfig`.
2. Initialize `tracing` subscriber.
3. Construct:

   * `EthClient`
   * `Wallet`
   * `UniswapV2Client`, `UniswapV3Client`
   * `BalanceService`, `PriceService`, `SwapService`
4. Start MCP server with registered tools and shared context.

The MCP server should expose tools:

* `"get_balance"`
* `"get_token_price"`
* `"swap_tokens"`

Each tool:

* Defines JSON schema for input/output (via `serde` + `rmcp`).
* Validates and converts inputs into domain types.
* Calls corresponding service method.
* Returns structured response or a user-friendly error message.

### 5.2 Tool Handlers → Services

Example: `get_balance` handler:

```rust
#[instrument(skip(ctx))]
pub async fn handle_get_balance(
    ctx: &AppContext,
    input: GetBalanceInput,
) -> Result<BalanceInfo, McpError> {
    let wallet = parse_address(&input.wallet_address)
        .map_err(McpError::invalid_argument)?;
    let token_addr = input
        .token_address
        .as_deref()
        .map(parse_address)
        .transpose()
        .map_err(McpError::invalid_argument)?;

    ctx.balance_service
        .get_balance(wallet, token_addr)
        .await
        .map_err(McpError::from)
}
```

---

## 6. Service Layer

### 6.1 BalanceService

Responsibilities:

* ETH balance via `eth_getBalance`.
* ERC20 balance via `balanceOf` and `decimals`/`symbol`.
* Normalize balances using `rust_decimal` and token decimals.

Dependencies:

* `EthClient` for RPC calls.
* Optional `TokenRegistry` (simple in-memory map of well-known token metadata).

Key operations:

* `get_balance(wallet: Address, token: Option<Address>) -> BalanceInfo`
* If no token, return ETH balance.
* If token provided:

  * Call `decimals()` and `symbol()` once and cache results in `DashMap<Address, TokenMetadata>`.

### 6.2 PriceService

Responsibilities:

* Resolve token identifier to address.
* Compute price in USD or ETH.
* Use on-chain data from Uniswap or an external API (behind `PriceSource` trait).

Design:

```rust
#[async_trait::async_trait]
pub trait PriceSource: Send + Sync {
    async fn get_price_in_eth(&self, token: Address) -> Result<Decimal, ServiceError>;
    async fn get_eth_price_in_usd(&self) -> Result<Decimal, ServiceError>;
}

pub struct PriceService<S: PriceSource> {
    source: S,
}
```

For first version:

* Implement `UniswapPriceSource`:

  * For `token/ETH`: query Uniswap V2 pair reserves (or V3 pool slot0 and liquidity).
  * Derive spot price via reserve ratios.
* Convert ETH price to USD via:

  * Another Uniswap pool (e.g. WETH/USDC) or
  * Configurable external API (optional for assignment, can stub).

### 6.3 SwapService

Responsibilities:

* Simulate token swaps on Uniswap V2/V3.
* Build realistic router transactions.
* Use `eth_call` to simulate and estimate output & gas.

Design:

```rust
pub struct SwapService {
    eth: EthClient,
    uni_v2: UniswapV2Client,
    uni_v3: UniswapV3Client,
    wallet: Wallet,
    default_slippage_bps: u32,
}

impl SwapService {
    pub async fn simulate_swap(
        &self,
        input: SwapTokensInput,
    ) -> Result<SwapSimulationResult, ServiceError> {
        // 1. Resolve tokens to addresses + metadata.
        // 2. Determine DEX (input.dex or default).
        // 3. Build router call (function signature and data).
        // 4. Run eth_call for quote and eth_estimateGas for gas.
        // 5. Convert raw results to Decimal and build SwapSimulationResult.
    }
}
```

---

## 7. Ethereum Integration Layer

### 7.1 EthClient

Encapsulates Alloy RPC client with async methods:

```rust
pub struct EthClient {
    provider: alloy_provider::Something, // exact type TBD
    chain_id: u64,
}

impl EthClient {
    pub async fn get_eth_balance(&self, addr: Address) -> Result<U256, EthError>;
    pub async fn call(&self, call: Eip1559CallRequest) -> Result<Bytes, EthError>;
    pub async fn estimate_gas(&self, call: Eip1559CallRequest) -> Result<u64, EthError>;
    pub async fn gas_price(&self) -> Result<U256, EthError>;
    pub async fn block_number(&self) -> Result<u64, EthError>;
}
```

Behavior:

* Encodes JSON-RPC calls using Alloy.
* Applies timeouts and simple retry policy for transient errors.
* Logs RPC calls with `tracing` at debug level (methods, targets, RPC id).

### 7.2 UniswapV2Client & UniswapV3Client

Responsibilities:

* Wrap Uniswap router & pair/pool interactions.
* Build calldata for swaps.

`UniswapV2Client`:

* Knows router address.
* Provides:

  * `get_amounts_out(amount_in, path) -> Result<Vec<U256>, ServiceError>`
  * `build_swap_exact_tokens_for_tokens(...) -> Eip1559CallRequest`

`UniswapV3Client`:

* Knows router address and pool fee tiers.
* Provides:

  * `quote_exact_input_single(...)` via Quoter contract.
  * `build_exact_input_single_tx(...)`.

Simulation flow:

1. Use Quoter (V3) or router `getAmountsOut` (V2) to compute output.
2. Build a router tx:

   * `to` = router address.
   * `data` = encoded function call for actual swap.
   * `from` = wallet address (derived from private key).
   * `value` = amount of ETH if `from_token` is ETH/WETH.
3. Call `eth.estimate_gas` and `eth.gas_price`.
4. Call `eth.call` with same tx; decode return value for `amountOut`.

---

## 8. Wallet & Signing

### 8.1 Wallet Module

```rust
pub struct Wallet {
    secret_key: alloy_signer::LocalWallet, // or similar
    address: Address,
    chain_id: u64,
}

impl Wallet {
    pub fn from_env(chain_id: u64) -> Result<Self, WalletError>;
    pub fn address(&self) -> Address;
    pub async fn sign_transaction(&self, tx: Eip1559Transaction) -> Result<Bytes, WalletError>;
}
```

Configuration:

* `ETH_PRIVATE_KEY` env var (hex).
* Optionally support `ETH_WALLET_MNEMONIC` in the future.

Usage in this project:

* For simulation:

  * Only `wallet.address()` is required to set the `from` field; `eth_call` doesn’t require real signatures.
* For future extension:

  * `sign_transaction` can be used with `eth_sendRawTransaction` (on-chain execution).

---

## 9. Configuration & Secrets

`AppConfig`:

```rust
pub struct AppConfig {
    pub rpc_url: String,
    pub chain_id: u64,
    pub default_network: String,          // "mainnet"
    pub uniswap_v2_router: Address,
    pub uniswap_v3_router: Address,
    pub uniswap_quoter_v3: Address,
    pub default_slippage_bps: u32,
}
```

Loading:

* Read from environment variables (e.g. `ETH_RPC_URL`, `CHAIN_ID`, router addresses).
* Optional `config.toml` support for local development.

Secrets:

* Private key only via env (`ETH_PRIVATE_KEY`).
* Avoid logging secrets; redact in error messages.
* Fail fast if private key is missing (or run in readonly mode with a config flag).

---

## 10. Logging & Observability

Use `tracing` for structured logs:

* Initialize subscriber in `main.rs`:

  * Default level: `INFO`, debug for specific modules.
* Use `#[instrument]` on important async functions (`services`, `mcp handlers`).
* Add fields:

  * Tool name.
  * Chain id / network.
  * Wallet address (shortened).
  * DEX kind.

Examples:

* Info logs for:

  * Tool invocations.
  * Swap simulations (from → to, amount).
* Debug logs for:

  * Raw RPC calls (method, target, duration).
* Error logs for:

  * RPC failures.
  * ABI decode errors.
  * Invalid inputs.

This keeps behavior easy to debug and extensions safe.

---

## 11. Error Handling Strategy

Central `Error` enums:

```rust
pub enum ServiceError {
    InvalidInput(String),
    RpcError(String),
    ContractError(String),
    InsufficientLiquidity,
    SlippageTooHigh,
    Internal(String),
}

pub enum McpError {
    InvalidArgument(String),
    FailedPrecondition(String),
    Internal(String),
}
```

Mapping:

* `ServiceError::InvalidInput` → `McpError::InvalidArgument`.
* `ServiceError::InsufficientLiquidity` → `FailedPrecondition`.
* Unexpected errors → `Internal`.

Each MCP tool response:

* Uses a structured error format with:

  * `code` – e.g. `"INVALID_ARGUMENT"`, `"RPC_ERROR"`, `"SIMULATION_FAILED"`.
  * `message` – human readable.
  * Optional `details` – for debugging or advanced clients.

---

## 12. Testing Strategy

### 12.1 Unit Tests

* **Pure logic**:

  * Token amount normalization (U256 ↔ Decimal, decimals).
  * Price calculations from reserves.
  * Slippage computation and minOut.
* **Error mapping**:

  * Service errors to MCP errors.

### 12.2 Integration Tests

Located in `tests/`:

* `integration_balance.rs`

  * `get_balance` for ETH and a known token (e.g. USDC) against real RPC.
* `integration_price.rs`

  * `get_token_price` for a well-known token.
* `integration_swap.rs`

  * `swap_tokens` simulation for small amounts with known routes.

Mark real-RPC tests:

* Use `#[ignore]` or a feature flag (`--features e2e`) so CI can control when they run.
* Require `ETH_RPC_URL` and `ETH_PRIVATE_KEY` to be set.

### 12.3 MCP Tool Tests

* Use a small harness to send JSON-RPC 2.0 requests to the MCP server.
* Validate JSON schemas and example flows from README.

---

## 13. Extensibility Considerations

This design aims to make future changes straightforward:

* **More tools**

  * E.g. `approve_token`, `get_open_orders`, `get_gas_quote`.
  * Implement as new handlers in `mcp/tools.rs` that call new service methods.

* **More DEXes**

  * Add `dex/sushiswap.rs`, implement a trait like `DexSwapProvider`.
  * `SwapService` can be refactored to depend on `Box<dyn DexSwapProvider>`.

* **More networks**

  * Add additional `EthClient` instances per network or a `MultiChainEthClient`.
  * `AppConfig` extended with multiple RPC URLs and router addresses.

* **More price sources**

  * Implement additional `PriceSource`s and choose by config:

    * On-chain only.
    * External API + on-chain fallback.
