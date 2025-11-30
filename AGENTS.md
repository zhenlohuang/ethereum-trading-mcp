# Repository Guidelines

## Project Structure & Module Organization
- Core code lives in `src/`: `main.rs` (binary entry), `lib.rs` (exports), `config` (env + settings), `ethereum` (RPC client, wallets, Uniswap/Chainlink contracts), `services` (balance/price/swap logic), `types` (shared structs), `mcp` (stdio server wiring), `error` (custom errors).
- Docs and references are in `docs/` (API reference, system design, screenshots). `mcp.dev.json` configures local MCP Inspector runs. Build artifacts land in `target/`.

## Setup, Build, and Run
- Copy `.env.dev` values into your shell and set `ETHEREUM_RPC_URL`, `ETHEREUM_PRIVATE_KEY`, optional `ETHEREUM_CHAIN_ID`/`LOG_LEVEL`.
- Build debug: `cargo build`; optimized: `cargo build --release`.
- Run server: `cargo run -p ethereum-trading-mcp` (reads env vars). Release binary: `./target/release/ethereum-trading-mcp`.
- Inspect via MCP Inspector: `npx @modelcontextprotocol/inspector --config mcp.dev.json -e ETHEREUM_RPC_URL=... -e ETHEREUM_PRIVATE_KEY=...`.

## Coding Style & Naming Conventions
- Rust 2021 edition; 4-space indent; keep lines ≤100 chars (`rustfmt.toml` enforces). Run `cargo fmt` before pushing.
- Imports are grouped per crate (`imports_granularity = "Crate"`); prefer `use` reorder enabled.
- Naming: `snake_case` for functions/vars, `UpperCamelCase` for types, `SCREAMING_SNAKE_CASE` for consts. Keep module files focused (one concern per file).

## Testing Guidelines
- Unit tests live alongside code under `#[cfg(test)]`; async tests use `tokio::test`/`tokio-test`.
- Run full suite: `cargo test`. Targeted: `cargo test services::swap`. Add tests for new services and error paths; keep RPC calls mocked/faked where possible.

## Commit & Pull Request Guidelines
- Follow conventional commits seen in history: `feat: ...`, `fix: ...`, `chore: ...`, `docs: ...` (optional scopes are fine).
- Keep commits focused and tidy; rebase noisy work before opening PRs.
- PRs should include: intent summary, linked issue (if any), test/Inspector commands run, and screenshots for user-facing docs or output changes.

## Security & Configuration Tips
- Never commit private keys or RPC URLs. Prefer `.env.dev` + local shell exports; avoid embedding secrets in tests.
- Use mainnet defaults (`ETHEREUM_CHAIN_ID=1`) unless explicitly testing forks; document when using alternative endpoints.
