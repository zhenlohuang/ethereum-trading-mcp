//! Price query service.

use alloy::primitives::{Address, U160, U256};
use rust_decimal::Decimal;
use std::{collections::HashMap, sync::Arc, time::SystemTime};

use crate::{
    error::{AppError, Result},
    ethereum::{
        contracts::{
            chainlink::{get_chainlink_feeds, IAggregatorV3},
            uniswap_v2::{IUniswapV2Factory, IUniswapV2Pair, UNISWAP_V2_FACTORY},
            uniswap_v3::{fee_tiers, IQuoterV2, UNISWAP_V3_QUOTER},
            WETH_ADDRESS,
        },
        EthereumClient,
    },
    services::BalanceService,
    types::{PriceInfo, PriceSource, QuoteCurrency, TokenInfo},
};

/// Get current Unix timestamp in seconds.
/// Returns 0 if system time is before Unix epoch (should never happen in practice).
fn current_timestamp() -> u64 {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Service for fetching token prices.
#[derive(Clone)]
pub struct PriceService {
    client: Arc<EthereumClient>,
    balance_service: BalanceService,
    chainlink_feeds: HashMap<Address, Address>,
}

impl PriceService {
    /// Create a new price service.
    pub fn new(client: Arc<EthereumClient>, balance_service: BalanceService) -> Self {
        Self { client, balance_service, chainlink_feeds: get_chainlink_feeds() }
    }

    /// Get token price in specified quote currency.
    pub async fn get_price(
        &self,
        token_address: Address,
        quote_currency: QuoteCurrency,
    ) -> Result<PriceInfo> {
        tracing::debug!(
            token = %token_address,
            quote = ?quote_currency,
            "Fetching token price"
        );

        // Get token metadata
        let metadata = self.balance_service.get_token_metadata(token_address).await?;

        // Try Chainlink first for USD prices
        if quote_currency == QuoteCurrency::USD {
            if let Some(feed_address) = self.chainlink_feeds.get(&token_address) {
                if let Ok(price_info) =
                    self.get_chainlink_price(token_address, *feed_address, &metadata.symbol).await
                {
                    return Ok(price_info);
                }
            }
        }

        // Fall back to Uniswap for price
        self.get_uniswap_price(token_address, quote_currency, &metadata.symbol, metadata.decimals)
            .await
    }

    /// Get price from Chainlink oracle.
    async fn get_chainlink_price(
        &self,
        token_address: Address,
        feed_address: Address,
        symbol: &str,
    ) -> Result<PriceInfo> {
        let contract = IAggregatorV3::new(feed_address, self.client.provider().clone());

        let round_data = contract.latestRoundData().call().await?;
        let decimals = contract.decimals().call().await?;

        // Chainlink returns price with 8 decimals typically
        // Convert I256 answer to i128 then to Decimal
        let answer_str = round_data.answer.to_string();
        let answer_i128: i128 = answer_str.parse().unwrap_or(0);
        let price = Decimal::from(answer_i128) / Decimal::from(10i64.pow(decimals as u32));

        Ok(PriceInfo {
            token: TokenInfo::erc20(token_address, symbol.to_string(), 18),
            price: price.to_string(),
            quote_currency: QuoteCurrency::USD,
            source: PriceSource::Chainlink,
            timestamp: current_timestamp(),
        })
    }

    /// Get price from Uniswap pools.
    async fn get_uniswap_price(
        &self,
        token_address: Address,
        quote_currency: QuoteCurrency,
        symbol: &str,
        decimals: u8,
    ) -> Result<PriceInfo> {
        // For ETH quote, use WETH pair
        // For USD quote, use USDC pair or WETH->USDC
        let quote_token = match quote_currency {
            QuoteCurrency::ETH => WETH_ADDRESS,
            QuoteCurrency::USD => {
                // Use USDC as USD proxy
                crate::ethereum::contracts::USDC_ADDRESS
            }
        };

        // Try V3 first with common fee tiers
        if let Ok(price) = self.get_uniswap_v3_price(token_address, quote_token, decimals).await {
            return Ok(PriceInfo {
                token: TokenInfo::erc20(token_address, symbol.to_string(), decimals),
                price: price.to_string(),
                quote_currency,
                source: PriceSource::UniswapV3,
                timestamp: current_timestamp(),
            });
        }

        // Fall back to V2
        if let Ok(price) = self.get_uniswap_v2_price(token_address, quote_token, decimals).await {
            return Ok(PriceInfo {
                token: TokenInfo::erc20(token_address, symbol.to_string(), decimals),
                price: price.to_string(),
                quote_currency,
                source: PriceSource::UniswapV2,
                timestamp: current_timestamp(),
            });
        }

        Err(AppError::PoolNotFound)
    }

    /// Get price from Uniswap V3.
    async fn get_uniswap_v3_price(
        &self,
        token_in: Address,
        token_out: Address,
        token_in_decimals: u8,
    ) -> Result<Decimal> {
        let quoter = IQuoterV2::new(UNISWAP_V3_QUOTER, self.client.provider().clone());

        // Try each fee tier
        for fee in fee_tiers::ALL_FEES {
            // Fee tiers are u32, convert to u24 (safe as all fee tiers are < 2^24)
            let fee_u24 = match u32::try_into(fee) {
                Ok(f) => f,
                Err(_) => continue, // Skip invalid fee tiers
            };
            let params = IQuoterV2::QuoteExactInputSingleParams {
                tokenIn: token_in,
                tokenOut: token_out,
                amountIn: U256::from(10u64.pow(token_in_decimals as u32)), // 1 token
                fee: fee_u24,
                sqrtPriceLimitX96: U160::ZERO,
            };

            if let Ok(result) = quoter.quoteExactInputSingle(params).call().await {
                // Convert to price (assuming 6 decimals for USDC, 18 for WETH)
                let out_decimals =
                    if token_out == crate::ethereum::contracts::USDC_ADDRESS { 6 } else { 18 };

                let amount_out_str = result.amountOut.to_string();
                let amount_out: u128 = amount_out_str.parse().unwrap_or(0);
                let price = Decimal::from(amount_out) / Decimal::from(10i64.pow(out_decimals));

                return Ok(price);
            }
        }

        Err(AppError::PoolNotFound)
    }

    /// Get price from Uniswap V2.
    async fn get_uniswap_v2_price(
        &self,
        token_in: Address,
        token_out: Address,
        token_in_decimals: u8,
    ) -> Result<Decimal> {
        let factory = IUniswapV2Factory::new(UNISWAP_V2_FACTORY, self.client.provider().clone());

        // getPair returns Address directly (tuple with single element)
        let pair_address: Address = factory.getPair(token_in, token_out).call().await?;

        if pair_address == Address::ZERO {
            return Err(AppError::PoolNotFound);
        }

        let pair = IUniswapV2Pair::new(pair_address, self.client.provider().clone());

        let reserves = pair.getReserves().call().await?;
        let token0: Address = pair.token0().call().await?;

        // Determine which reserve is which
        let (reserve_in, reserve_out) = if token0 == token_in {
            (reserves.reserve0, reserves.reserve1)
        } else {
            (reserves.reserve1, reserves.reserve0)
        };

        // Calculate price
        let out_decimals =
            if token_out == crate::ethereum::contracts::USDC_ADDRESS { 6 } else { 18 };

        // Convert U112 reserves to u128 for Decimal
        let reserve_in_u128: u128 = reserve_in.to_string().parse().unwrap_or(0);
        let reserve_out_u128: u128 = reserve_out.to_string().parse().unwrap_or(0);

        // Price = (reserve_out / 10^out_decimals) / (reserve_in / 10^in_decimals)
        let price = Decimal::from(reserve_out_u128)
            * Decimal::from(10i64.pow(token_in_decimals as u32))
            / Decimal::from(reserve_in_u128)
            / Decimal::from(10i64.pow(out_decimals));

        Ok(price)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chainlink_feeds_contains_common_tokens() {
        let feeds = get_chainlink_feeds();
        // Should contain ETH, BTC, USDC feeds
        assert!(feeds.contains_key(&WETH_ADDRESS));
    }

    #[test]
    fn test_quote_currency_parsing() {
        assert_eq!("USD".parse::<QuoteCurrency>().unwrap(), QuoteCurrency::USD);
        assert_eq!("ETH".parse::<QuoteCurrency>().unwrap(), QuoteCurrency::ETH);
        assert_eq!("usd".parse::<QuoteCurrency>().unwrap(), QuoteCurrency::USD);
        assert!("INVALID".parse::<QuoteCurrency>().is_err());
    }

    #[test]
    fn test_price_calculation_from_reserves() {
        // Simulating a pool with 1000 token_in and 2000 token_out
        let reserve_in: u128 = 1_000_000_000_000_000_000_000; // 1000 * 10^18
        let reserve_out: u128 = 2_000_000_000; // 2000 * 10^6 (USDC)

        let token_in_decimals = 18u8;
        let out_decimals = 6u32;

        // Price = (reserve_out / 10^6) / (reserve_in / 10^18)
        let price = Decimal::from(reserve_out) * Decimal::from(10i64.pow(token_in_decimals as u32))
            / Decimal::from(reserve_in)
            / Decimal::from(10i64.pow(out_decimals));

        // Expected: 2000 / 1000 = 2.0
        assert_eq!(price, Decimal::from(2));
    }
}
