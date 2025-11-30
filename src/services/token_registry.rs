//! Token Registry service with remote fetching and caching.
//!
//! Fetches token information from Uniswap Token Lists and caches them
//! for efficient lookups.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use alloy::primitives::Address;
use serde::Deserialize;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::error::{AppError, Result};

// ============================================================================
// Token List Sources
// ============================================================================

/// Uniswap default token list URL.
pub const UNISWAP_TOKEN_LIST_URL: &str = "https://tokens.uniswap.org";

/// 1inch token list URL (alternative source).
pub const ONE_INCH_TOKEN_LIST_URL: &str = "https://tokens.1inch.eth.limo";

/// Default cache TTL (24 hours).
pub const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(86400);

// ============================================================================
// Token List Types (following tokenlists.org schema)
// ============================================================================

/// Token information from token list.
#[derive(Debug, Clone, Deserialize)]
pub struct TokenListToken {
    /// Chain ID where the token exists.
    #[serde(rename = "chainId")]
    pub chain_id: u64,
    /// Token contract address.
    pub address: String,
    /// Token symbol (e.g., "USDC").
    pub symbol: String,
    /// Token name (e.g., "USD Coin").
    pub name: String,
    /// Number of decimals.
    pub decimals: u8,
    /// Logo URI (optional).
    #[serde(rename = "logoURI")]
    pub logo_uri: Option<String>,
}

/// Token list response from API.
#[derive(Debug, Clone, Deserialize)]
pub struct TokenListResponse {
    /// List name.
    pub name: String,
    /// List of tokens.
    pub tokens: Vec<TokenListToken>,
}

// ============================================================================
// Cached Token Entry
// ============================================================================

/// A token entry with parsed address.
#[derive(Debug, Clone)]
pub struct TokenEntry {
    /// Token contract address.
    pub address: Address,
    /// Token symbol.
    pub symbol: String,
    /// Token name.
    pub name: String,
    /// Number of decimals.
    pub decimals: u8,
    /// Chain ID.
    pub chain_id: u64,
}

// ============================================================================
// Token Registry
// ============================================================================

/// Cache state for token registry.
struct CacheState {
    /// Tokens indexed by (chain_id, symbol_uppercase).
    by_symbol: HashMap<(u64, String), TokenEntry>,
    /// Tokens indexed by (chain_id, address).
    by_address: HashMap<(u64, Address), TokenEntry>,
    /// Last update timestamp.
    last_updated: Option<Instant>,
}

impl CacheState {
    fn new() -> Self {
        Self { by_symbol: HashMap::new(), by_address: HashMap::new(), last_updated: None }
    }

    fn is_expired(&self, ttl: Duration) -> bool {
        match self.last_updated {
            Some(last) => last.elapsed() > ttl,
            None => true,
        }
    }
}

/// Token Registry with caching support.
///
/// Provides token lookups by symbol or address with:
/// - Remote fetching from Uniswap Token Lists
/// - In-memory caching with 24-hour TTL
/// - Auto-refresh on cache miss
pub struct TokenRegistry {
    /// HTTP client for fetching token lists.
    client: reqwest::Client,
    /// Token list URL.
    token_list_url: String,
    /// Target chain ID.
    chain_id: u64,
    /// Cache TTL.
    cache_ttl: Duration,
    /// Cached token data.
    cache: Arc<RwLock<CacheState>>,
}

impl TokenRegistry {
    /// Create a new TokenRegistry.
    ///
    /// # Arguments
    /// * `chain_id` - Target chain ID (1 for mainnet, 11155111 for Sepolia, etc.)
    ///
    /// # Errors
    /// Returns an error if the HTTP client cannot be created.
    pub fn new(chain_id: u64) -> Result<Self> {
        Self::with_options(chain_id, UNISWAP_TOKEN_LIST_URL.to_string(), DEFAULT_CACHE_TTL)
    }

    /// Create a TokenRegistry with custom options.
    ///
    /// # Arguments
    /// * `chain_id` - Target chain ID
    /// * `token_list_url` - URL to fetch token list from
    /// * `cache_ttl` - Cache time-to-live (default: 24 hours)
    ///
    /// # Errors
    /// Returns an error if the HTTP client cannot be created.
    pub fn with_options(
        chain_id: u64,
        token_list_url: String,
        cache_ttl: Duration,
    ) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| AppError::Transport(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            token_list_url,
            chain_id,
            cache_ttl,
            cache: Arc::new(RwLock::new(CacheState::new())),
        })
    }

    /// Refresh the token cache from remote source.
    pub async fn refresh(&self) -> Result<usize> {
        info!("Refreshing token list from {}", self.token_list_url);

        let response = self
            .client
            .get(&self.token_list_url)
            .send()
            .await
            .map_err(|e| AppError::Transport(format!("Failed to fetch token list: {}", e)))?;

        if !response.status().is_success() {
            return Err(AppError::Transport(format!(
                "Token list API returned status: {}",
                response.status()
            )));
        }

        let token_list: TokenListResponse = response
            .json()
            .await
            .map_err(|e| AppError::Parse(format!("Failed to parse token list: {}", e)))?;

        let mut cache_guard = self.cache.write().await;
        let mut count = 0;

        for token in token_list.tokens {
            // Only include tokens for our target chain
            if token.chain_id != self.chain_id {
                continue;
            }

            // Parse address
            let address = match token.address.parse::<Address>() {
                Ok(addr) => addr,
                Err(e) => {
                    warn!("Invalid token address {}: {}", token.address, e);
                    continue;
                }
            };

            let entry = TokenEntry {
                address,
                symbol: token.symbol.clone(),
                name: token.name,
                decimals: token.decimals,
                chain_id: token.chain_id,
            };

            let symbol_key = (token.chain_id, token.symbol.to_uppercase());
            let address_key = (token.chain_id, address);

            cache_guard.by_symbol.insert(symbol_key, entry.clone());
            cache_guard.by_address.insert(address_key, entry);
            count += 1;
        }

        cache_guard.last_updated = Some(Instant::now());
        info!("Loaded {} tokens for chain {}", count, self.chain_id);

        Ok(count)
    }

    /// Ensure cache is fresh, refreshing if needed.
    async fn ensure_fresh(&self) -> Result<()> {
        let needs_refresh = {
            let cache_guard = self.cache.read().await;
            cache_guard.is_expired(self.cache_ttl)
        };

        if needs_refresh {
            self.refresh().await?;
        }
        Ok(())
    }

    /// Resolve a token symbol to an address.
    ///
    /// If the token is not found in cache, forces a refresh and retries.
    ///
    /// # Arguments
    /// * `symbol` - Token symbol (case-insensitive, e.g., "USDC", "weth")
    ///
    /// # Returns
    /// Token entry if found, None otherwise.
    pub async fn resolve_symbol(&self, symbol: &str) -> Option<TokenEntry> {
        // First, ensure cache is fresh
        if let Err(e) = self.ensure_fresh().await {
            warn!("Failed to refresh token list: {}", e);
        }

        let key = (self.chain_id, symbol.to_uppercase());

        // Try to find in cache
        {
            let cache_guard = self.cache.read().await;
            if let Some(entry) = cache_guard.by_symbol.get(&key) {
                return Some(entry.clone());
            }
        }

        // Not found - force refresh and retry
        info!("Token '{}' not found in cache, forcing refresh", symbol);
        if let Err(e) = self.refresh().await {
            warn!("Failed to refresh token list on cache miss: {}", e);
            return None;
        }

        // Retry after refresh
        let cache_guard = self.cache.read().await;
        cache_guard.by_symbol.get(&key).cloned()
    }

    /// Look up a token by address.
    ///
    /// If the token is not found in cache, forces a refresh and retries.
    ///
    /// # Arguments
    /// * `address` - Token contract address
    ///
    /// # Returns
    /// Token entry if found, None otherwise.
    pub async fn lookup_address(&self, address: Address) -> Option<TokenEntry> {
        // First, ensure cache is fresh
        if let Err(e) = self.ensure_fresh().await {
            warn!("Failed to refresh token list: {}", e);
        }

        let key = (self.chain_id, address);

        // Try to find in cache
        {
            let cache_guard = self.cache.read().await;
            if let Some(entry) = cache_guard.by_address.get(&key) {
                return Some(entry.clone());
            }
        }

        // Not found - force refresh and retry
        info!("Token address {:?} not found in cache, forcing refresh", address);
        if let Err(e) = self.refresh().await {
            warn!("Failed to refresh token list on cache miss: {}", e);
            return None;
        }

        // Retry after refresh
        let cache_guard = self.cache.read().await;
        cache_guard.by_address.get(&key).cloned()
    }

    /// Get address for a symbol (convenience method).
    pub async fn get_address(&self, symbol: &str) -> Option<Address> {
        self.resolve_symbol(symbol).await.map(|t| t.address)
    }

    /// Get all cached tokens for the current chain.
    pub async fn list_tokens(&self) -> Vec<TokenEntry> {
        if let Err(e) = self.ensure_fresh().await {
            warn!("Failed to refresh token list: {}", e);
        }

        let cache_guard = self.cache.read().await;
        cache_guard.by_symbol.values().filter(|t| t.chain_id == self.chain_id).cloned().collect()
    }

    /// Get cache statistics.
    pub async fn cache_stats(&self) -> (usize, Option<Duration>) {
        let cache_guard = self.cache.read().await;
        let count = cache_guard.by_symbol.len();
        let age = cache_guard.last_updated.map(|t| t.elapsed());
        (count, age)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_state_is_expired() {
        let state = CacheState::new();
        // New cache without last_updated should be expired
        assert!(state.is_expired(Duration::from_secs(3600)));
    }

    #[test]
    fn test_registry_creation() {
        let registry = TokenRegistry::new(1).expect("Failed to create registry");
        assert_eq!(registry.chain_id, 1);
        assert_eq!(registry.cache_ttl, DEFAULT_CACHE_TTL);
    }

    #[test]
    fn test_registry_with_custom_options() {
        let registry = TokenRegistry::with_options(
            42,
            "https://custom.tokens.api".to_string(),
            Duration::from_secs(7200),
        )
        .expect("Failed to create registry");
        assert_eq!(registry.chain_id, 42);
        assert_eq!(registry.cache_ttl, Duration::from_secs(7200));
    }
}
