//! Token Registry service with remote fetching and caching.
//!
//! Fetches token information from Uniswap Token Lists and caches them
//! for efficient lookups.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use alloy::primitives::Address;
use async_trait::async_trait;
use serde::Deserialize;
use tokio::sync::{RwLock, Semaphore};
use tracing::{info, warn};

use crate::error::{AppError, Result};
use alloy::primitives::address;

use crate::ethereum::contracts::{USDC_ADDRESS, WBTC_ADDRESS, WETH_ADDRESS};

/// UNI token address (mainnet).
const UNI_ADDRESS: Address = address!("1f9840a85d5aF5bf1D1762F925BDADdC4201F984");

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
// Token Registry Trait
// ============================================================================

/// Trait for token registry operations.
///
/// Provides an abstraction for token lookups, allowing different implementations
/// (e.g., remote fetching, local caching, or mock implementations for testing).
#[async_trait]
pub trait TokenRegistryTrait: Send + Sync {
    /// Resolve a token symbol to a token entry.
    ///
    /// # Arguments
    /// * `symbol` - Token symbol (case-insensitive, e.g., "USDC", "weth")
    ///
    /// # Returns
    /// Token entry if found, None otherwise.
    async fn resolve_symbol(&self, symbol: &str) -> Option<TokenEntry>;

    /// Look up a token by address.
    ///
    /// # Arguments
    /// * `address` - Token contract address
    ///
    /// # Returns
    /// Token entry if found, None otherwise.
    async fn lookup_address(&self, address: Address) -> Option<TokenEntry>;
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

    /// Insert a token entry into both indexes.
    fn insert(&mut self, entry: TokenEntry) {
        let symbol_key = (entry.chain_id, entry.symbol.to_uppercase());
        let address_key = (entry.chain_id, entry.address);
        self.by_symbol.insert(symbol_key, entry.clone());
        self.by_address.insert(address_key, entry);
    }
}

/// Token Registry with caching support.
///
/// Provides token lookups by symbol or address with:
/// - Remote fetching from Uniswap Token Lists
/// - In-memory caching with 24-hour TTL
/// - Auto-refresh on cache miss
/// - Concurrent refresh protection (only one refresh at a time)
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
    /// Semaphore to prevent concurrent cache refreshes.
    refresh_semaphore: Semaphore,
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

        let registry = Self {
            client,
            token_list_url,
            chain_id,
            cache_ttl,
            cache: Arc::new(RwLock::new(CacheState::new())),
            refresh_semaphore: Semaphore::new(1),
        };

        // Pre-populate with well-known mainnet tokens as fallback
        if chain_id == 1 {
            registry.populate_fallback_tokens();
        }

        Ok(registry)
    }

    /// Pre-populate cache with well-known mainnet tokens.
    /// These serve as fallbacks when remote token list is unavailable.
    fn populate_fallback_tokens(&self) {
        let fallback_tokens = vec![
            TokenEntry {
                address: WETH_ADDRESS,
                symbol: "WETH".to_string(),
                name: "Wrapped Ether".to_string(),
                decimals: 18,
                chain_id: 1,
            },
            TokenEntry {
                address: USDC_ADDRESS,
                symbol: "USDC".to_string(),
                name: "USD Coin".to_string(),
                decimals: 6,
                chain_id: 1,
            },
            TokenEntry {
                address: WBTC_ADDRESS,
                symbol: "WBTC".to_string(),
                name: "Wrapped BTC".to_string(),
                decimals: 8,
                chain_id: 1,
            },
            TokenEntry {
                address: UNI_ADDRESS,
                symbol: "UNI".to_string(),
                name: "Uniswap".to_string(),
                decimals: 18,
                chain_id: 1,
            },
        ];

        // Use try_write to avoid blocking - this is best-effort
        if let Ok(mut cache_guard) = self.cache.try_write() {
            let count = fallback_tokens.len();
            for token in fallback_tokens {
                cache_guard.insert(token);
            }
            info!("Pre-populated {} fallback tokens for mainnet", count);
        }
    }

    /// Ensure cache is fresh, refreshing if needed.
    ///
    /// Uses double-check locking pattern with a semaphore to prevent
    /// multiple concurrent refresh operations.
    async fn ensure_fresh(&self) -> Result<()> {
        // First check without acquiring the semaphore
        let needs_refresh = {
            let cache_guard = self.cache.read().await;
            cache_guard.is_expired(self.cache_ttl)
        };

        if needs_refresh {
            // Acquire semaphore to prevent concurrent refreshes
            let _permit = self.refresh_semaphore.acquire().await.map_err(|_| {
                AppError::Transport("Failed to acquire refresh semaphore".to_string())
            })?;

            // Double-check: another task may have refreshed while we waited
            let still_needs_refresh = {
                let cache_guard = self.cache.read().await;
                cache_guard.is_expired(self.cache_ttl)
            };

            if still_needs_refresh {
                self.refresh().await?;
            }
        }
        Ok(())
    }

    /// Refresh the token cache from remote source.
    ///
    /// # Returns
    /// The number of tokens loaded into the cache.
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
    ///
    /// # Returns
    /// A tuple of (token count, cache age).
    pub async fn cache_stats(&self) -> (usize, Option<Duration>) {
        let cache_guard = self.cache.read().await;
        let count = cache_guard.by_symbol.len();
        let age = cache_guard.last_updated.map(|t| t.elapsed());
        (count, age)
    }
}

#[async_trait]
impl TokenRegistryTrait for TokenRegistry {
    async fn resolve_symbol(&self, symbol: &str) -> Option<TokenEntry> {
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

    async fn lookup_address(&self, address: Address) -> Option<TokenEntry> {
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
