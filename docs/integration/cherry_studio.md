# Cherry Studio Integration

## Overview

Integrate the Ethereum Trading MCP Server into Cherry Studio.

## Steps

1. Add the Ethereum Trading MCP Server to Cherry Studio

``` json
{
    "mcpServers": {
        "ethereum-trading-mcp": {
            "type": "stdio",
            "command": "/path/to/ethereum-trading-mcp",
            "args": [],
            "env": {
                "ETHEREUM_RPC_URL": "https://mainnet.infura.io/v3/YOUR_API_KEY",
                "ETHEREUM_PRIVATE_KEY": "0x...",
                "LOG_LEVEL": "info"
            }
        }
    }
}
```

![](./images/cherry-studio/import-mcp-json.png)

Please confirm the MCP server is configured correctly.

![](./images/cherry-studio/add-mcp-server.png)

2. Create a new chatbot with the Ethereum Trading MCP Server

![](./images/cherry-studio/create-eth-assistant.png)
![](./images/cherry-studio/enable-mcp-in-eth-assistant.png)

3. Test the integration

![](./images/cherry-studio/test-mcp-server.png)
