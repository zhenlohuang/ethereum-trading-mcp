//! Wallet management.

use alloy::{primitives::Address, signers::local::PrivateKeySigner};

use crate::error::{AppError, Result};

/// Wallet manager for transaction signing.
#[derive(Clone)]
pub struct WalletManager {
    /// The local signer.
    signer: PrivateKeySigner,
    /// Wallet address.
    address: Address,
}

impl WalletManager {
    /// Create a wallet manager from a private key string.
    pub fn from_private_key(private_key: &str) -> Result<Self> {
        // Remove 0x prefix if present
        let key = private_key.strip_prefix("0x").unwrap_or(private_key);

        let signer: PrivateKeySigner =
            key.parse().map_err(|e: alloy::signers::local::LocalSignerError| {
                AppError::Wallet(e.to_string())
            })?;

        let address = signer.address();

        tracing::info!(address = %address, "Wallet initialized");

        Ok(Self { signer, address })
    }

    /// Get the wallet address.
    pub fn address(&self) -> Address {
        self.address
    }

    /// Get the signer for transaction signing.
    pub fn signer(&self) -> &PrivateKeySigner {
        &self.signer
    }
}

impl std::fmt::Debug for WalletManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WalletManager").field("address", &self.address).finish()
    }
}
