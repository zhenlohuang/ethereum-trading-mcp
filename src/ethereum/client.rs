//! Ethereum RPC client.

use alloy::{
    network::Ethereum,
    primitives::{Address, Bytes, U256},
    providers::{Provider, ProviderBuilder, RootProvider},
    rpc::types::TransactionRequest,
};
use std::sync::Arc;
use tokio::sync::OnceCell;

use crate::error::{AppError, Result};

/// Type alias for the HTTP provider.
pub type HttpProvider = RootProvider<Ethereum>;

/// Ethereum RPC client wrapper with lazy initialization.
#[derive(Clone)]
pub struct EthereumClient {
    /// The underlying provider.
    provider: Arc<HttpProvider>,
    /// RPC URL for logging.
    rpc_url: String,
    /// Lazily initialized chain ID.
    chain_id: Arc<OnceCell<u64>>,
}

impl EthereumClient {
    /// Create a new Ethereum client.
    ///
    /// Note: This does NOT make any network calls. The connection is
    /// established lazily when the first operation is performed.
    pub fn new(rpc_url: &str) -> Result<Self> {
        let url = rpc_url
            .parse()
            .map_err(|_| AppError::Config(format!("Invalid RPC URL: {}", rpc_url)))?;

        #[allow(deprecated)]
        let provider = ProviderBuilder::new().on_http(url).root().clone();

        tracing::info!(rpc_url = %rpc_url, "Ethereum client created (lazy initialization)");

        Ok(Self {
            provider: Arc::new(provider),
            rpc_url: rpc_url.to_string(),
            chain_id: Arc::new(OnceCell::new()),
        })
    }

    /// Get the chain ID (fetches from network on first call).
    pub async fn chain_id(&self) -> Result<u64> {
        self.chain_id
            .get_or_try_init(|| async {
                let chain_id = self.provider.get_chain_id().await?;
                tracing::info!(chain_id = chain_id, rpc_url = %self.rpc_url, "Connected to Ethereum node");
                Ok(chain_id)
            })
            .await
            .copied()
    }

    /// Get the underlying provider.
    pub fn provider(&self) -> &HttpProvider {
        &self.provider
    }

    /// Get native ETH balance for an address.
    pub async fn get_eth_balance(&self, address: Address) -> Result<U256> {
        let balance = self.provider.get_balance(address).await?;
        Ok(balance)
    }

    /// Execute a call (simulate transaction without broadcasting).
    pub async fn call(&self, tx: &TransactionRequest) -> Result<Bytes> {
        let result = self.provider.call(tx.clone()).await?;
        Ok(result)
    }

    /// Estimate gas for a transaction.
    pub async fn estimate_gas(&self, tx: &TransactionRequest) -> Result<u64> {
        let gas = self.provider.estimate_gas(tx.clone()).await?;
        Ok(gas)
    }

    /// Get current gas price.
    pub async fn get_gas_price(&self) -> Result<u128> {
        let gas_price = self.provider.get_gas_price().await?;
        Ok(gas_price)
    }

    /// Get the current block timestamp.
    pub async fn get_block_timestamp(&self) -> Result<u64> {
        let block = self
            .provider
            .get_block_by_number(alloy::eips::BlockNumberOrTag::Latest)
            .await?
            .ok_or_else(|| AppError::Rpc("Failed to get latest block".into()))?;
        Ok(block.header.timestamp)
    }

    /// Make a contract call.
    pub async fn call_contract(
        &self,
        to: Address,
        data: Bytes,
        value: Option<U256>,
    ) -> Result<Bytes> {
        let mut tx = TransactionRequest::default().to(to).input(data.into());

        if let Some(v) = value {
            tx = tx.value(v);
        }

        self.call(&tx).await
    }
}
