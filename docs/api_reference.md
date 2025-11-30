# API Reference

## get_balance

Query ETH or ERC20 token balance for a wallet address.

**Request:**
```json
{
  "method": "tools/call",
  "params": {
    "name": "get_balance",
    "arguments": {
      "address": "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
      "token_address": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
    }
  }
}
```

**Response:**
```json
{
  "address": "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
  "token": {
    "address": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
    "symbol": "USDC",
    "decimals": 6
  },
  "balance": "1234.567890",
  "balance_raw": "1234567890"
}
```

## get_token_price

Get current token price from on-chain sources.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `token` | string | Yes | Token symbol (e.g., "WETH", "USDC", "UNI") |
| `quote_currency` | string | No | "USD" or "ETH" (default: "USD") |

**Request:**
```json
{
  "method": "tools/call",
  "params": {
    "name": "get_token_price",
    "arguments": {
      "token": "WETH",
      "quote_currency": "USD"
    }
  }
}
```

**Response:**
```json
{
  "token": {
    "address": "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
    "symbol": "WETH",
    "decimals": 18
  },
  "price": "2500.50",
  "quote_currency": "USD",
  "source": "chainlink",
  "timestamp": 1699999999
}
```

## swap_tokens

Simulate a token swap on Uniswap V2/V3.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `from_token` | string | Yes | Input token symbol (e.g., "WETH", "USDC") |
| `to_token` | string | Yes | Output token symbol (e.g., "WETH", "USDC") |
| `amount` | string | Yes | Amount to swap (human-readable, e.g., "1.5") |
| `slippage_tolerance` | number | No | Slippage tolerance percentage (default: 0.5) |

**Request:**
```json
{
  "method": "tools/call",
  "params": {
    "name": "swap_tokens",
    "arguments": {
      "from_token": "WETH",
      "to_token": "USDC",
      "amount": "1.0",
      "slippage_tolerance": 0.5
    }
  }
}
```

**Response:**
```json
{
  "simulation_success": true,
  "simulation_error": null,
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

**Response (simulation failed):**
```json
{
  "simulation_success": false,
  "simulation_error": "Insufficient token balance or allowance",
  "amount_in": "1.0",
  "amount_out_expected": "2500.123456",
  "amount_out_minimum": "2487.622789",
  "price_impact": "0.05",
  "gas_estimate": "200000",
  "gas_price": "30000000000",
  "gas_cost_eth": "0.006",
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
