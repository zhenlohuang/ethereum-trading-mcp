//! Ethereum interaction module.
//!
//! Contains the Ethereum client, wallet management, and contract bindings.

pub mod client;
pub mod contracts;
pub mod wallet;

pub use client::{EthereumClient, HttpProvider};
pub use wallet::WalletManager;
