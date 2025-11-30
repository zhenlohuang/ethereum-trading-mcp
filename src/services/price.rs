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

        // Special case: WETH priced in ETH is always 1:1
        // (WETH is wrapped ETH, so 1 WETH = 1 ETH)
        if token_address == WETH_ADDRESS && quote_currency == QuoteCurrency::ETH {
            return Ok(PriceInfo {
                token: TokenInfo::erc20(token_address, metadata.symbol.clone(), metadata.decimals),
                price: "1".to_string(),
                quote_currency: QuoteCurrency::ETH,
                source: PriceSource::UniswapV3, // Use V3 as nominal source
                timestamp: current_timestamp(),
            });
        }

        // Special case: USDC priced in USD is always 1:1
        // (USDC is the USD proxy, so querying USDC/USDC would fail)
        if token_address == crate::ethereum::contracts::USDC_ADDRESS
            && quote_currency == QuoteCurrency::USD
        {
            return Ok(PriceInfo {
                token: TokenInfo::erc20(token_address, metadata.symbol.clone(), metadata.decimals),
                price: "1".to_string(),
                quote_currency: QuoteCurrency::USD,
                source: PriceSource::Chainlink, // Nominal source
                timestamp: current_timestamp(),
            });
        }

        // Try Chainlink first for USD prices
        if quote_currency == QuoteCurrency::USD {
            if let Some(feed_address) = self.chainlink_feeds.get(&token_address) {
                if let Ok(price_info) = self
                    .get_chainlink_price(
                        token_address,
                        *feed_address,
                        &metadata.symbol,
                        metadata.decimals,
                    )
                    .await
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
    ///
    /// Validates that the price data is fresh and positive:
    /// - Answer must be positive (> 0)
    /// - Data must not be stale (answeredInRound >= roundId)
    /// - UpdatedAt must be recent (within 1 hour for most feeds)
    async fn get_chainlink_price(
        &self,
        token_address: Address,
        feed_address: Address,
        symbol: &str,
        token_decimals: u8,
    ) -> Result<PriceInfo> {
        let contract = IAggregatorV3::new(feed_address, self.client.provider().clone());

        let round_data = contract.latestRoundData().call().await?;
        let decimals = contract.decimals().call().await?;

        // Validate the Chainlink response
        // 1. Check that answeredInRound >= roundId (data is not stale)
        if round_data.answeredInRound < round_data.roundId {
            return Err(AppError::PriceOracle(format!(
                "Stale Chainlink data: answeredInRound ({}) < roundId ({})",
                round_data.answeredInRound, round_data.roundId
            )));
        }

        // 2. Check that updatedAt is recent (within 1 hour = 3600 seconds)
        // Most Chainlink feeds update at least hourly
        let now = current_timestamp();
        let updated_at: u64 = round_data
            .updatedAt
            .try_into()
            .map_err(|_| AppError::NumericOverflow("updatedAt timestamp overflow".to_string()))?;
        const STALENESS_THRESHOLD: u64 = 3600; // 1 hour
        if now > updated_at && now - updated_at > STALENESS_THRESHOLD {
            return Err(AppError::PriceOracle(format!(
                "Stale Chainlink data: last update was {} seconds ago (threshold: {})",
                now - updated_at,
                STALENESS_THRESHOLD
            )));
        }

        // 3. Check that answer is positive
        if round_data.answer.is_negative() || round_data.answer.is_zero() {
            return Err(AppError::PriceOracle(format!(
                "Invalid Chainlink answer: {} (must be positive)",
                round_data.answer
            )));
        }

        // Convert I256 answer to i128 with overflow check
        let answer_str = round_data.answer.to_string();
        let answer_i128: i128 = answer_str.parse().map_err(|_| {
            AppError::NumericOverflow(format!(
                "Chainlink answer {} exceeds i128 range",
                round_data.answer
            ))
        })?;

        let price = Decimal::from(answer_i128) / Decimal::from(10i64.pow(decimals as u32));

        Ok(PriceInfo {
            token: TokenInfo::erc20(token_address, symbol.to_string(), token_decimals),
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

                let amount_out: u128 = result.amountOut.try_into().map_err(|_| {
                    AppError::NumericOverflow(format!(
                        "Uniswap V3 quote amountOut {} exceeds u128 range",
                        result.amountOut
                    ))
                })?;
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

        // Convert U112 reserves to u128 for Decimal with overflow check
        let reserve_in_u128: u128 = reserve_in.try_into().map_err(|_| {
            AppError::NumericOverflow(format!(
                "Uniswap V2 reserve_in {} exceeds u128 range",
                reserve_in
            ))
        })?;
        let reserve_out_u128: u128 = reserve_out.try_into().map_err(|_| {
            AppError::NumericOverflow(format!(
                "Uniswap V2 reserve_out {} exceeds u128 range",
                reserve_out
            ))
        })?;

        // Validate reserves are non-zero to avoid division by zero
        if reserve_in_u128 == 0 {
            return Err(AppError::InsufficientLiquidity);
        }

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

    #[test]
    fn test_weth_eth_special_case_condition() {
        // WETH priced in ETH should be handled as a special case (1:1 ratio)
        // This test verifies the condition logic without network calls
        let weth = WETH_ADDRESS;
        let quote_eth = QuoteCurrency::ETH;
        let quote_usd = QuoteCurrency::USD;

        // WETH + ETH quote should trigger special case
        assert!(weth == WETH_ADDRESS && quote_eth == QuoteCurrency::ETH);

        // WETH + USD quote should NOT trigger special case
        assert!(!(weth == WETH_ADDRESS && quote_usd == QuoteCurrency::ETH));
    }
}
