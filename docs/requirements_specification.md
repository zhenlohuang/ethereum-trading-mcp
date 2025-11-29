## Overview

Build a Model Context Protocol (MCP) server in Rust that enables AI agents to query balances and execute token swaps on Ethereum.

## Requirements

### Core Functionality

Implement an MCP server with the following tools:

1. **`get_balance`** - Query ETH and ERC20 token balances
    - Input: wallet address, optional token contract address
    - Output: balance information with proper decimals
2. **`get_token_price`** - Get current token price in USD or ETH
    - Input: token address or symbol
    - Output: price data
3. **`swap_tokens`** - Execute a token swap on Uniswap V2 or V3
    - Input: from_token, to_token, amount, slippage tolerance
    - Output: simulation result showing estimated output and gas costs
    - **Important**: Construct a real Uniswap transaction and submit it to the blockchain for simulation (using `eth_call` or similar). The transaction should NOT be executed on-chain.

### Technical Stack

**Required:**

- Rust with async runtime (tokio)
- Ethereum RPC client library (ethers-rs or alloy)
- MCP SDK for Rust ([rmcp](https://github.com/modelcontextprotocol/rust-sdk)) or implement JSON-RPC 2.0 manually
- Structured logging (tracing)

### Constraints

- Must connect to real Ethereum RPC (use public endpoints or Infura/Alchemy)
- Balance queries must fetch real on-chain data
- For swaps: construct real Uniswap V2/V3 swap transactions and simulate them using RPC methods
- Transaction signing: implement basic wallet management (e.g., private key via environment variable or config file)
- Use `rust_decimal` or similar for financial precision

## Deliverables

1. **Working code** - Rust project that compiles and runs
2. **README** with:
    - Setup instructions (dependencies, env vars, how to run)
    - Example MCP tool call (show JSON request/response)
    - Design decisions (3-5 sentences on your approach)
    - Known limitations or assumptions
3. **Tests** - Demonstrate core functionality

## Development Approach

You're **encouraged** to use AI assistants (Cursor, Claude Code, GitHub Copilot, etc.) while working on this assignment. However, the solution should demonstrate your understanding of:

- Rust and async programming
- Ethereum fundamentals
- System design and architecture

The code will be reviewed for comprehension and design decisions.

## Submission

Create a GitHub repository and share the link. Ensure:

- `cargo build` compiles successfully
- `cargo test` passes
- README has clear setup instructions
- Code is well-organized and readable
