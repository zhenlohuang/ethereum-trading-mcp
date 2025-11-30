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

#[cfg(test)]
mod tests {
    use super::*;

    // A valid test private key (DO NOT use in production!)
    // This is a well-known test key from Hardhat/Foundry
    const TEST_PRIVATE_KEY: &str =
        "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
    const TEST_PRIVATE_KEY_NO_PREFIX: &str =
        "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

    #[test]
    fn test_wallet_from_private_key_with_prefix() {
        let wallet = WalletManager::from_private_key(TEST_PRIVATE_KEY);
        assert!(wallet.is_ok());

        let wallet = wallet.unwrap();
        // The first Hardhat account address (compare case-insensitively)
        let addr_str = format!("{:?}", wallet.address()).to_lowercase();
        assert_eq!(addr_str, "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266");
    }

    #[test]
    fn test_wallet_from_private_key_without_prefix() {
        let wallet = WalletManager::from_private_key(TEST_PRIVATE_KEY_NO_PREFIX);
        assert!(wallet.is_ok());

        let wallet = wallet.unwrap();
        let addr_str = format!("{:?}", wallet.address()).to_lowercase();
        assert_eq!(addr_str, "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266");
    }

    #[test]
    fn test_wallet_invalid_private_key() {
        // Too short
        let result = WalletManager::from_private_key("0x1234");
        assert!(result.is_err());

        // Invalid hex
        let result = WalletManager::from_private_key("0xZZZZ");
        assert!(result.is_err());

        // Empty
        let result = WalletManager::from_private_key("");
        assert!(result.is_err());
    }

    #[test]
    fn test_wallet_address_getter() {
        let wallet = WalletManager::from_private_key(TEST_PRIVATE_KEY).unwrap();
        let address = wallet.address();

        // Address should be non-zero
        assert_ne!(address, Address::ZERO);
    }

    #[test]
    fn test_wallet_signer_getter() {
        let wallet = WalletManager::from_private_key(TEST_PRIVATE_KEY).unwrap();
        let signer = wallet.signer();

        // Signer should return the same address
        assert_eq!(signer.address(), wallet.address());
    }

    #[test]
    fn test_wallet_debug_trait() {
        let wallet = WalletManager::from_private_key(TEST_PRIVATE_KEY).unwrap();
        let debug_str = format!("{:?}", wallet);

        // Debug should contain "WalletManager" and the address
        assert!(debug_str.contains("WalletManager"));
        assert!(debug_str.contains("address"));
        // Should NOT contain the private key
        assert!(
            !debug_str.contains("ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
        );
    }

    #[test]
    fn test_wallet_clone() {
        let wallet1 = WalletManager::from_private_key(TEST_PRIVATE_KEY).unwrap();
        let wallet2 = wallet1.clone();

        assert_eq!(wallet1.address(), wallet2.address());
    }

    #[test]
    fn test_wallet_different_keys_different_addresses() {
        // Second Hardhat test account
        let key2 = "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d";

        let wallet1 = WalletManager::from_private_key(TEST_PRIVATE_KEY).unwrap();
        let wallet2 = WalletManager::from_private_key(key2).unwrap();

        assert_ne!(wallet1.address(), wallet2.address());
    }

    #[test]
    fn test_wallet_error_contains_message() {
        let result = WalletManager::from_private_key("invalid_key");
        assert!(result.is_err());

        if let Err(e) = result {
            // Error should be a Wallet variant
            match e {
                AppError::Wallet(msg) => {
                    assert!(!msg.is_empty());
                }
                _ => panic!("Expected Wallet error"),
            }
        }
    }
}
