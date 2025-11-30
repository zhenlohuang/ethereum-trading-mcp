//! Balance query service.

use alloy::primitives::Address;
use std::sync::Arc;

use crate::{
    error::Result,
    ethereum::{
        contracts::erc20::{TokenMetadata, IERC20},
        EthereumClient,
    },
    types::{format_units, BalanceInfo, TokenInfo},
};

/// Service for querying token balances.
#[derive(Clone)]
pub struct BalanceService {
    client: Arc<EthereumClient>,
}

impl BalanceService {
    /// Create a new balance service.
    pub fn new(client: Arc<EthereumClient>) -> Self {
        Self { client }
    }

    /// Get balance for an address.
    ///
    /// If `token_address` is None, returns native ETH balance.
    /// Otherwise, returns ERC20 token balance.
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

    /// Get native ETH balance.
    async fn get_eth_balance(&self, address: Address) -> Result<BalanceInfo> {
        tracing::debug!(address = %address, "Querying ETH balance");

        let balance = self.client.get_eth_balance(address).await?;
        let formatted = format_units(balance, 18);

        Ok(BalanceInfo {
            address: format!("{address:?}"),
            token: TokenInfo::eth(),
            balance: formatted,
            balance_raw: balance.to_string(),
        })
    }

    /// Get ERC20 token balance.
    async fn get_erc20_balance(&self, address: Address, token: Address) -> Result<BalanceInfo> {
        tracing::debug!(
            address = %address,
            token = %token,
            "Querying ERC20 balance"
        );

        // Get token metadata
        let metadata = self.get_token_metadata(token).await?;

        // Get balance - balanceOf returns U256 directly
        let contract = IERC20::new(token, self.client.provider().clone());
        let balance = contract.balanceOf(address).call().await?;

        let formatted = format_units(balance, metadata.decimals);

        Ok(BalanceInfo {
            address: format!("{address:?}"),
            token: TokenInfo::erc20(token, metadata.symbol, metadata.decimals),
            balance: formatted,
            balance_raw: balance.to_string(),
        })
    }

    /// Get token metadata (symbol, decimals).
    pub async fn get_token_metadata(&self, token: Address) -> Result<TokenMetadata> {
        let contract = IERC20::new(token, self.client.provider().clone());

        // Get symbol - returns String directly
        let symbol = contract.symbol().call().await.unwrap_or_else(|_| "UNKNOWN".to_string());

        // Get name - returns String directly
        let name = contract.name().call().await.unwrap_or_else(|_| "Unknown Token".to_string());

        // Get decimals - returns u8 directly
        let decimals = contract.decimals().call().await.unwrap_or(18);

        Ok(TokenMetadata { name, symbol, decimals, address: token })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::U256;

    #[test]
    fn test_token_info_eth() {
        let info = TokenInfo::eth();
        assert_eq!(info.symbol, "ETH");
        assert_eq!(info.decimals, 18);
        assert!(info.address.is_none());
    }

    #[test]
    fn test_token_info_erc20() {
        let addr = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse::<Address>().unwrap();
        let info = TokenInfo::erc20(addr, "USDC".to_string(), 6);
        assert_eq!(info.symbol, "USDC");
        assert_eq!(info.decimals, 6);
        assert!(info.address.is_some());
    }

    #[test]
    fn test_balance_info_formatting() {
        let balance = U256::from(1_000_000_000_000_000_000u64); // 1 ETH
        let formatted = format_units(balance, 18);
        assert_eq!(formatted, "1");

        let balance_usdc = U256::from(1_500_000u64); // 1.5 USDC
        let formatted_usdc = format_units(balance_usdc, 6);
        assert_eq!(formatted_usdc, "1.5");
    }
}
