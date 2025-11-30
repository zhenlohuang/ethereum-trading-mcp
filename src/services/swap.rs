//! Swap simulation service.

use alloy::{
    primitives::{aliases::U24, Address, Bytes, U160, U256},
    rpc::types::TransactionRequest,
    sol_types::SolCall,
};
use rust_decimal::Decimal;
use std::sync::Arc;
use std::time::SystemTime;

use crate::{
    error::{AppError, Result},
    ethereum::{
        contracts::{
            uniswap_v2::{
                IUniswapV2Factory, IUniswapV2Router02, UNISWAP_V2_FACTORY, UNISWAP_V2_ROUTER,
            },
            uniswap_v3::{
                fee_tiers, IQuoterV2, ISwapRouter, IUniswapV3Factory, UNISWAP_V3_FACTORY,
                UNISWAP_V3_QUOTER, UNISWAP_V3_ROUTER,
            },
            WETH_ADDRESS,
        },
        EthereumClient, WalletManager,
    },
    services::BalanceService,
    types::{
        format_units, SwapParams, SwapRoute, SwapSimulationResult, TransactionData, UniswapVersion,
    },
};

/// Get current Unix timestamp in seconds.
/// Returns 0 if system time is before Unix epoch (should never happen in practice).
fn current_timestamp() -> u64 {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Service for simulating token swaps.
#[derive(Clone)]
pub struct SwapService {
    client: Arc<EthereumClient>,
    wallet: WalletManager,
    balance_service: BalanceService,
}

impl SwapService {
    /// Create a new swap service.
    pub fn new(
        client: Arc<EthereumClient>,
        wallet: WalletManager,
        balance_service: BalanceService,
    ) -> Self {
        Self { client, wallet, balance_service }
    }

    /// Simulate a token swap.
    pub async fn simulate_swap(&self, params: SwapParams) -> Result<SwapSimulationResult> {
        tracing::info!(
            from = %params.from_token,
            to = %params.to_token,
            amount = %params.amount_in,
            slippage = %params.slippage_tolerance,
            "Simulating swap"
        );

        // Get token metadata for formatting
        let from_metadata = self.balance_service.get_token_metadata(params.from_token).await?;
        let to_metadata = self.balance_service.get_token_metadata(params.to_token).await?;

        // Try V3 first, then V2
        let (route, amount_out, tx) = match self.try_v3_swap(&params).await {
            Ok(result) => result,
            Err(_) => {
                // Try V2
                self.try_v2_swap(&params).await?
            }
        };

        // Calculate minimum output with slippage
        let slippage_multiplier = Decimal::ONE - params.slippage_tolerance / Decimal::from(100);
        let amount_out_u128: u128 = amount_out.try_into().map_err(|_| {
            AppError::NumericOverflow(format!("amount_out {} exceeds u128 range", amount_out))
        })?;
        let amount_out_min = Decimal::from(amount_out_u128) * slippage_multiplier;
        let amount_out_min_u128: u128 = Self::decimal_to_u128(amount_out_min)?;
        let amount_out_min_u256 = U256::from(amount_out_min_u128);

        // Simulate the transaction using eth_call to verify it would execute
        let (simulation_success, simulation_error) = match self.simulate_transaction(&tx).await {
            Ok(()) => {
                tracing::info!("Swap simulation successful - transaction would execute");
                (true, None)
            }
            Err(error_msg) => {
                tracing::warn!(error = %error_msg, "Swap simulation failed - transaction would revert");
                (false, Some(error_msg))
            }
        };

        // Estimate gas (may fail if simulation failed, use default in that case)
        let gas_estimate = self.estimate_gas(&tx).await.unwrap_or(200_000);
        let gas_price = self.client.get_gas_price().await.unwrap_or(30_000_000_000);

        // Calculate gas cost in ETH
        let gas_cost_wei = U256::from(gas_estimate) * U256::from(gas_price);
        let gas_cost_eth = format_units(gas_cost_wei, 18);

        // Calculate price impact by comparing spot price vs execution price
        let price_impact =
            self.calculate_price_impact(&params, amount_out, &route).await.unwrap_or(Decimal::ZERO);

        // Format amounts
        let amount_in_formatted = format_units(params.amount_in, from_metadata.decimals);
        let amount_out_formatted = format_units(amount_out, to_metadata.decimals);
        let amount_out_min_formatted = format_units(amount_out_min_u256, to_metadata.decimals);

        // Build transaction data
        let tx_data = TransactionData {
            to: tx.to.and_then(|t| t.to().map(|addr| format!("{:?}", addr))).unwrap_or_default(),
            data: tx
                .input
                .input()
                .map(|d| format!("0x{}", alloy::hex::encode(d)))
                .unwrap_or_default(),
            value: tx.value.map(|v| v.to_string()).unwrap_or_else(|| "0".to_string()),
        };

        Ok(SwapSimulationResult {
            simulation_success,
            simulation_error,
            amount_in: amount_in_formatted,
            amount_out_expected: amount_out_formatted,
            amount_out_minimum: amount_out_min_formatted,
            price_impact: price_impact.to_string(),
            gas_estimate: gas_estimate.to_string(),
            gas_price: gas_price.to_string(),
            gas_cost_eth,
            route,
            transaction: tx_data,
        })
    }

    /// Try to build a V3 swap.
    async fn try_v3_swap(
        &self,
        params: &SwapParams,
    ) -> Result<(SwapRoute, U256, TransactionRequest)> {
        let factory = IUniswapV3Factory::new(UNISWAP_V3_FACTORY, self.client.provider().clone());
        let quoter = IQuoterV2::new(UNISWAP_V3_QUOTER, self.client.provider().clone());

        // Find best fee tier
        let mut best_fee: Option<u32> = None;
        let mut best_amount_out = U256::ZERO;

        for fee in fee_tiers::ALL_FEES {
            // Check if pool exists - getPool returns Address directly
            // fee is u32, convert to U24 for the contract call
            let fee_u24 = U24::from(fee);
            let pool: Address =
                factory.getPool(params.from_token, params.to_token, fee_u24).call().await?;

            if pool == Address::ZERO {
                continue;
            }

            // Get quote
            let quote_params = IQuoterV2::QuoteExactInputSingleParams {
                tokenIn: params.from_token,
                tokenOut: params.to_token,
                amountIn: params.amount_in,
                fee: fee_u24,
                sqrtPriceLimitX96: U160::ZERO,
            };

            if let Ok(result) = quoter.quoteExactInputSingle(quote_params).call().await {
                if result.amountOut > best_amount_out {
                    best_amount_out = result.amountOut;
                    best_fee = Some(fee);
                }
            }
        }

        let fee = best_fee.ok_or(AppError::PoolNotFound)?;

        if best_amount_out == U256::ZERO {
            return Err(AppError::InsufficientLiquidity);
        }

        // Build swap transaction
        let deadline = params.deadline.unwrap_or_else(|| current_timestamp() + 1200); // 20 minutes

        // Calculate minimum amount out with slippage
        let slippage_multiplier = Decimal::ONE - params.slippage_tolerance / Decimal::from(100);
        let best_amount_out_u128: u128 = best_amount_out.try_into().map_err(|_| {
            AppError::NumericOverflow(format!(
                "best_amount_out {} exceeds u128 range",
                best_amount_out
            ))
        })?;
        let min_out = Decimal::from(best_amount_out_u128) * slippage_multiplier;
        let min_out_u128: u128 = Self::decimal_to_u128(min_out)?;
        let amount_out_min = U256::from(min_out_u128);

        // Build swap params with fee converted to U24
        let swap_params = ISwapRouter::ExactInputSingleParams {
            tokenIn: params.from_token,
            tokenOut: params.to_token,
            fee: U24::from(fee),
            recipient: self.wallet.address(),
            deadline: U256::from(deadline),
            amountIn: params.amount_in,
            amountOutMinimum: amount_out_min,
            sqrtPriceLimitX96: U160::ZERO,
        };

        let calldata = ISwapRouter::exactInputSingleCall { params: swap_params }.abi_encode();

        let tx = TransactionRequest::default()
            .to(UNISWAP_V3_ROUTER)
            .input(Bytes::from(calldata).into())
            .from(self.wallet.address());

        let route = SwapRoute {
            protocol: UniswapVersion::V3,
            path: vec![format!("{:?}", params.from_token), format!("{:?}", params.to_token)],
            fee_tier: Some(fee),
        };

        Ok((route, best_amount_out, tx))
    }

    /// Try to build a V2 swap.
    async fn try_v2_swap(
        &self,
        params: &SwapParams,
    ) -> Result<(SwapRoute, U256, TransactionRequest)> {
        let factory = IUniswapV2Factory::new(UNISWAP_V2_FACTORY, self.client.provider().clone());
        let router = IUniswapV2Router02::new(UNISWAP_V2_ROUTER, self.client.provider().clone());

        // Check if pair exists - getPair returns Address directly
        let pair: Address = factory.getPair(params.from_token, params.to_token).call().await?;

        if pair == Address::ZERO {
            // Try routing through WETH
            let pair_a: Address = factory.getPair(params.from_token, WETH_ADDRESS).call().await?;
            let pair_b: Address = factory.getPair(WETH_ADDRESS, params.to_token).call().await?;

            if pair_a == Address::ZERO || pair_b == Address::ZERO {
                return Err(AppError::PoolNotFound);
            }

            // Route through WETH
            return self.build_v2_multihop_swap(params).await;
        }

        // Get amounts out - returns Vec<U256> directly
        let path = vec![params.from_token, params.to_token];
        let amounts: Vec<U256> =
            router.getAmountsOut(params.amount_in, path.clone()).call().await?;

        let amount_out = amounts[1];

        if amount_out == U256::ZERO {
            return Err(AppError::InsufficientLiquidity);
        }

        // Build swap transaction
        let deadline = params.deadline.unwrap_or_else(|| current_timestamp() + 1200);

        // Calculate minimum amount out with slippage
        let slippage_multiplier = Decimal::ONE - params.slippage_tolerance / Decimal::from(100);
        let amount_out_u128: u128 = amount_out.try_into().map_err(|_| {
            AppError::NumericOverflow(format!("amount_out {} exceeds u128 range", amount_out))
        })?;
        let min_out = Decimal::from(amount_out_u128) * slippage_multiplier;
        let min_out_u128: u128 = Self::decimal_to_u128(min_out)?;
        let amount_out_min = U256::from(min_out_u128);

        let calldata = IUniswapV2Router02::swapExactTokensForTokensCall {
            amountIn: params.amount_in,
            amountOutMin: amount_out_min,
            path,
            to: self.wallet.address(),
            deadline: U256::from(deadline),
        }
        .abi_encode();

        let tx = TransactionRequest::default()
            .to(UNISWAP_V2_ROUTER)
            .input(Bytes::from(calldata).into())
            .from(self.wallet.address());

        let route = SwapRoute {
            protocol: UniswapVersion::V2,
            path: vec![format!("{:?}", params.from_token), format!("{:?}", params.to_token)],
            fee_tier: None,
        };

        Ok((route, amount_out, tx))
    }

    /// Build a V2 swap routing through WETH.
    async fn build_v2_multihop_swap(
        &self,
        params: &SwapParams,
    ) -> Result<(SwapRoute, U256, TransactionRequest)> {
        let router = IUniswapV2Router02::new(UNISWAP_V2_ROUTER, self.client.provider().clone());

        let path = vec![params.from_token, WETH_ADDRESS, params.to_token];
        let amounts: Vec<U256> =
            router.getAmountsOut(params.amount_in, path.clone()).call().await?;

        let amount_out = amounts[2];

        if amount_out == U256::ZERO {
            return Err(AppError::InsufficientLiquidity);
        }

        let deadline = params.deadline.unwrap_or_else(|| current_timestamp() + 1200);

        let slippage_multiplier = Decimal::ONE - params.slippage_tolerance / Decimal::from(100);
        let amount_out_u128: u128 = amount_out.try_into().map_err(|_| {
            AppError::NumericOverflow(format!(
                "multihop amount_out {} exceeds u128 range",
                amount_out
            ))
        })?;
        let min_out = Decimal::from(amount_out_u128) * slippage_multiplier;
        let min_out_u128: u128 = Self::decimal_to_u128(min_out)?;
        let amount_out_min = U256::from(min_out_u128);

        let calldata = IUniswapV2Router02::swapExactTokensForTokensCall {
            amountIn: params.amount_in,
            amountOutMin: amount_out_min,
            path: path.clone(),
            to: self.wallet.address(),
            deadline: U256::from(deadline),
        }
        .abi_encode();

        let tx = TransactionRequest::default()
            .to(UNISWAP_V2_ROUTER)
            .input(Bytes::from(calldata).into())
            .from(self.wallet.address());

        let route = SwapRoute {
            protocol: UniswapVersion::V2,
            path: path.iter().map(|a| format!("{:?}", a)).collect(),
            fee_tier: None,
        };

        Ok((route, amount_out, tx))
    }

    /// Estimate gas for a transaction.
    async fn estimate_gas(&self, tx: &TransactionRequest) -> Result<u64> {
        self.client.estimate_gas(tx).await
    }

    /// Simulate a transaction using eth_call to verify it would execute successfully.
    ///
    /// Returns Ok(()) if the simulation succeeds, or an error message if it fails.
    async fn simulate_transaction(
        &self,
        tx: &TransactionRequest,
    ) -> std::result::Result<(), String> {
        match self.client.call(tx).await {
            Ok(_) => {
                tracing::debug!("Transaction simulation successful");
                Ok(())
            }
            Err(e) => {
                let error_msg = e.to_string();
                tracing::warn!(error = %error_msg, "Transaction simulation failed");

                // Parse common revert reasons for better error messages
                let user_friendly_error = if error_msg.contains("insufficient") {
                    "Insufficient token balance or allowance".to_string()
                } else if error_msg.contains("INSUFFICIENT_OUTPUT_AMOUNT") {
                    "Output amount is less than minimum (slippage exceeded)".to_string()
                } else if error_msg.contains("EXPIRED") {
                    "Transaction deadline expired".to_string()
                } else if error_msg.contains("TRANSFER_FROM_FAILED") {
                    "Token transfer failed - check token approval".to_string()
                } else if error_msg.contains("execution reverted") {
                    format!("Transaction would revert: {}", error_msg)
                } else {
                    format!("Simulation failed: {}", error_msg)
                };

                Err(user_friendly_error)
            }
        }
    }

    /// Calculate approximate price impact by comparing spot price vs execution price.
    ///
    /// Price impact measures how much the trade size affects the execution price.
    /// A higher price impact means the trade is moving the market more significantly.
    async fn calculate_price_impact(
        &self,
        params: &SwapParams,
        amount_out: U256,
        route: &SwapRoute,
    ) -> Result<Decimal> {
        // Use a small reference amount to approximate the spot price
        // This gives us the "marginal" exchange rate without significant price impact
        let reference_amount = Self::calculate_reference_amount(params.amount_in);

        let spot_output = match route.protocol {
            UniswapVersion::V3 => {
                self.get_v3_quote(params, reference_amount, route.fee_tier).await?
            }
            UniswapVersion::V2 => self.get_v2_quote(params, reference_amount).await?,
        };

        // Calculate rates (output per unit of input)
        // spot_rate = spot_output / reference_amount
        // execution_rate = amount_out / amount_in
        //
        // Price impact = (1 - execution_rate / spot_rate) * 100
        //              = (1 - (amount_out * reference_amount) / (spot_output * amount_in)) * 100

        // Convert U256 values to u128 with overflow checking
        // For price impact calculation, overflow indicates extremely large values
        // which would likely result in very high price impact anyway
        let amount_in_u128: u128 = params.amount_in.try_into().map_err(|_| {
            AppError::NumericOverflow(format!("amount_in {} exceeds u128 range", params.amount_in))
        })?;
        let amount_out_u128: u128 = amount_out.try_into().map_err(|_| {
            AppError::NumericOverflow(format!("amount_out {} exceeds u128 range", amount_out))
        })?;
        let reference_u128: u128 = reference_amount.try_into().map_err(|_| {
            AppError::NumericOverflow(format!(
                "reference_amount {} exceeds u128 range",
                reference_amount
            ))
        })?;
        let spot_output_u128: u128 = spot_output.try_into().map_err(|_| {
            AppError::NumericOverflow(format!("spot_output {} exceeds u128 range", spot_output))
        })?;

        // Avoid division by zero
        if spot_output_u128 == 0 || amount_in_u128 == 0 {
            return Ok(Decimal::ZERO);
        }

        // Use high precision decimals for the calculation
        // Calculate rate_ratio = (amount_out / amount_in) / (spot_output / reference)
        // To avoid overflow when multiplying large numbers, we divide first
        let amount_out_dec = Decimal::from(amount_out_u128);
        let amount_in_dec = Decimal::from(amount_in_u128);
        let spot_output_dec = Decimal::from(spot_output_u128);
        let reference_dec = Decimal::from(reference_u128);

        // execution_rate = amount_out / amount_in
        let execution_rate = amount_out_dec / amount_in_dec;

        // spot_rate = spot_output / reference
        let spot_rate = spot_output_dec / reference_dec;

        if spot_rate.is_zero() {
            return Ok(Decimal::ZERO);
        }

        // rate_ratio = execution_rate / spot_rate
        let rate_ratio = execution_rate / spot_rate;

        // Price impact = (1 - rate_ratio) * 100, ensure non-negative
        let price_impact = (Decimal::ONE - rate_ratio) * Decimal::from(100);
        let price_impact = price_impact.max(Decimal::ZERO);

        // Round to 4 decimal places
        Ok(price_impact.round_dp(4))
    }

    /// Calculate a small reference amount for spot price approximation.
    /// Uses 0.1% of the actual amount, with minimum and maximum bounds.
    fn calculate_reference_amount(amount_in: U256) -> U256 {
        // Use 0.1% of input amount as reference
        let reference = amount_in / U256::from(1000);

        // Set reasonable bounds
        let min_reference = U256::from(1_000u64); // Minimum to avoid dust amounts
        let max_reference = amount_in / U256::from(10); // Max 10% of input

        if reference < min_reference {
            min_reference.min(amount_in) // Don't exceed the actual input
        } else if reference > max_reference {
            max_reference
        } else {
            reference
        }
    }

    /// Convert a Decimal to u128 with overflow checking.
    /// Truncates to integer and validates it fits in u128.
    fn decimal_to_u128(value: Decimal) -> Result<u128> {
        let truncated = value.trunc();
        // Decimal's to_string for truncated value should be a valid integer
        truncated
            .to_string()
            .parse::<u128>()
            .map_err(|_| AppError::NumericOverflow(format!("Decimal {} exceeds u128 range", value)))
    }

    /// Get a V3 quote for a given amount.
    async fn get_v3_quote(
        &self,
        params: &SwapParams,
        amount_in: U256,
        fee_tier: Option<u32>,
    ) -> Result<U256> {
        let quoter = IQuoterV2::new(UNISWAP_V3_QUOTER, self.client.provider().clone());

        let fee = fee_tier.unwrap_or(3000); // Default to 0.3% tier
        let fee_u24 = U24::from(fee);

        let quote_params = IQuoterV2::QuoteExactInputSingleParams {
            tokenIn: params.from_token,
            tokenOut: params.to_token,
            amountIn: amount_in,
            fee: fee_u24,
            sqrtPriceLimitX96: U160::ZERO,
        };

        let result = quoter.quoteExactInputSingle(quote_params).call().await?;
        Ok(result.amountOut)
    }

    /// Get a V2 quote for a given amount.
    async fn get_v2_quote(&self, params: &SwapParams, amount_in: U256) -> Result<U256> {
        let router = IUniswapV2Router02::new(UNISWAP_V2_ROUTER, self.client.provider().clone());

        // Try direct path first
        let path = vec![params.from_token, params.to_token];
        match router.getAmountsOut(amount_in, path).call().await {
            Ok(amounts) => Ok(amounts[1]),
            Err(_) => {
                // Try routing through WETH
                let path_via_weth = vec![params.from_token, WETH_ADDRESS, params.to_token];
                let amounts = router.getAmountsOut(amount_in, path_via_weth).call().await?;
                Ok(amounts[2])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::format_units;

    #[test]
    fn test_slippage_calculation() {
        let amount_out = U256::from(1_000_000u64); // 1 USDC
        let slippage = Decimal::new(5, 1); // 0.5%

        let slippage_multiplier = Decimal::ONE - slippage / Decimal::from(100);
        let amount_out_u128: u128 = amount_out.to_string().parse().unwrap();
        let min_out = Decimal::from(amount_out_u128) * slippage_multiplier;

        // 0.5% slippage means minimum is 99.5% of original
        let expected = Decimal::from(995_000u64); // 0.995 * 1_000_000
        assert_eq!(min_out, expected);
    }

    #[test]
    fn test_deadline_default() {
        let now = current_timestamp();
        let deadline = now + 1200; // 20 minutes

        // Deadline should be 20 minutes (1200 seconds) in the future
        assert_eq!(deadline - now, 1200);
    }

    #[test]
    fn test_gas_cost_calculation() {
        let gas_estimate: u64 = 150_000;
        let gas_price: u128 = 30_000_000_000; // 30 gwei

        let gas_cost_wei = U256::from(gas_estimate) * U256::from(gas_price);
        let gas_cost_eth = format_units(gas_cost_wei, 18);

        // 150,000 * 30 gwei = 4,500,000 gwei = 0.0045 ETH
        assert_eq!(gas_cost_eth, "0.0045");
    }

    #[test]
    fn test_swap_route_creation() {
        let route = SwapRoute {
            protocol: UniswapVersion::V3,
            path: vec!["0xToken1".to_string(), "0xToken2".to_string()],
            fee_tier: Some(3000),
        };

        assert_eq!(route.protocol, UniswapVersion::V3);
        assert_eq!(route.path.len(), 2);
        assert_eq!(route.fee_tier, Some(3000));
    }

    // ============================================================================
    // current_timestamp Tests
    // ============================================================================

    #[test]
    fn test_current_timestamp_is_reasonable() {
        let ts = current_timestamp();
        // Should be after Jan 1, 2024 (1704067200)
        assert!(ts > 1704067200, "Timestamp should be after 2024");
        // Should be before year 2100 (4102444800)
        assert!(ts < 4102444800, "Timestamp should be before 2100");
    }

    #[test]
    fn test_current_timestamp_increases() {
        let ts1 = current_timestamp();
        // Small delay to ensure time passes
        std::thread::sleep(std::time::Duration::from_millis(10));
        let ts2 = current_timestamp();
        // ts2 should be >= ts1 (might be equal if within same second)
        assert!(ts2 >= ts1);
    }

    // ============================================================================
    // calculate_reference_amount Tests
    // ============================================================================

    #[test]
    fn test_calculate_reference_amount_normal() {
        // 1000 tokens -> 0.1% = 1 token reference
        let amount = U256::from(1_000_000_000_000_000_000_000u128); // 1000 * 10^18
        let reference = SwapService::calculate_reference_amount(amount);

        // Should be 0.1% = 1 token worth
        let expected = amount / U256::from(1000);
        assert_eq!(reference, expected);
    }

    #[test]
    fn test_calculate_reference_amount_minimum() {
        // Very small amount should use minimum (1000)
        let amount = U256::from(100u64);
        let reference = SwapService::calculate_reference_amount(amount);

        // Should be capped at amount itself (since 100 < 1000)
        assert_eq!(reference, amount);
    }

    #[test]
    fn test_calculate_reference_amount_large() {
        // Very large amount - 10% cap
        let amount = U256::from(1_000_000_000_000_000_000_000_000u128); // 1M tokens
        let reference = SwapService::calculate_reference_amount(amount);

        // 0.1% of 1M = 1000, which should be between min and max
        let expected = amount / U256::from(1000);
        let max_reference = amount / U256::from(10);

        assert!(reference <= max_reference);
        assert_eq!(reference, expected);
    }

    #[test]
    fn test_calculate_reference_amount_boundary() {
        // Test at boundary where 0.1% equals minimum
        let min_reference = U256::from(1_000u64);
        let amount = min_reference * U256::from(1000); // 1,000,000

        let reference = SwapService::calculate_reference_amount(amount);
        assert_eq!(reference, min_reference);
    }

    // ============================================================================
    // decimal_to_u128 Tests
    // ============================================================================

    #[test]
    fn test_decimal_to_u128_whole_number() {
        let value = Decimal::from(1000);
        let result = SwapService::decimal_to_u128(value).unwrap();
        assert_eq!(result, 1000u128);
    }

    #[test]
    fn test_decimal_to_u128_truncates_decimals() {
        let value = Decimal::new(12345, 2); // 123.45
        let result = SwapService::decimal_to_u128(value).unwrap();
        assert_eq!(result, 123u128); // Truncated
    }

    #[test]
    fn test_decimal_to_u128_zero() {
        let value = Decimal::ZERO;
        let result = SwapService::decimal_to_u128(value).unwrap();
        assert_eq!(result, 0u128);
    }

    #[test]
    fn test_decimal_to_u128_max_valid() {
        // Large but valid u128 value
        let value = Decimal::from(u64::MAX);
        let result = SwapService::decimal_to_u128(value).unwrap();
        assert_eq!(result, u64::MAX as u128);
    }

    // ============================================================================
    // Slippage Calculation Tests
    // ============================================================================

    #[test]
    fn test_slippage_1_percent() {
        let amount_out = U256::from(1_000_000_000u64); // 1B units
        let slippage = Decimal::from(1); // 1%

        let slippage_multiplier = Decimal::ONE - slippage / Decimal::from(100);
        let amount_out_u128: u128 = amount_out.try_into().unwrap();
        let min_out = Decimal::from(amount_out_u128) * slippage_multiplier;

        // 1% slippage = 99% of original
        let expected = Decimal::from(990_000_000u64);
        assert_eq!(min_out, expected);
    }

    #[test]
    fn test_slippage_5_percent() {
        let amount_out = U256::from(100_000_000u64);
        let slippage = Decimal::from(5); // 5%

        let slippage_multiplier = Decimal::ONE - slippage / Decimal::from(100);
        let amount_out_u128: u128 = amount_out.try_into().unwrap();
        let min_out = Decimal::from(amount_out_u128) * slippage_multiplier;

        // 5% slippage = 95% of original
        let expected = Decimal::from(95_000_000u64);
        assert_eq!(min_out, expected);
    }

    #[test]
    fn test_slippage_small() {
        let amount_out = U256::from(1_000_000u64);
        let slippage = Decimal::new(1, 1); // 0.1%

        let slippage_multiplier = Decimal::ONE - slippage / Decimal::from(100);
        let amount_out_u128: u128 = amount_out.try_into().unwrap();
        let min_out = Decimal::from(amount_out_u128) * slippage_multiplier;

        // 0.1% slippage = 99.9% of original
        let expected = Decimal::from(999_000u64);
        assert_eq!(min_out, expected);
    }

    // ============================================================================
    // Gas Cost Calculation Tests
    // ============================================================================

    #[test]
    fn test_gas_cost_high_gas_price() {
        let gas_estimate: u64 = 200_000;
        let gas_price: u128 = 100_000_000_000; // 100 gwei

        let gas_cost_wei = U256::from(gas_estimate) * U256::from(gas_price);
        let gas_cost_eth = format_units(gas_cost_wei, 18);

        // 200,000 * 100 gwei = 20,000,000 gwei = 0.02 ETH
        assert_eq!(gas_cost_eth, "0.02");
    }

    #[test]
    fn test_gas_cost_low_gas_price() {
        let gas_estimate: u64 = 100_000;
        let gas_price: u128 = 5_000_000_000; // 5 gwei

        let gas_cost_wei = U256::from(gas_estimate) * U256::from(gas_price);
        let gas_cost_eth = format_units(gas_cost_wei, 18);

        // 100,000 * 5 gwei = 500,000 gwei = 0.0005 ETH
        assert_eq!(gas_cost_eth, "0.0005");
    }

    // ============================================================================
    // SwapRoute Tests
    // ============================================================================

    #[test]
    fn test_swap_route_v2_direct() {
        let route = SwapRoute {
            protocol: UniswapVersion::V2,
            path: vec!["WETH".to_string(), "USDC".to_string()],
            fee_tier: None,
        };

        assert_eq!(route.protocol, UniswapVersion::V2);
        assert_eq!(route.path.len(), 2);
        assert!(route.fee_tier.is_none());
    }

    #[test]
    fn test_swap_route_v2_multihop() {
        let route = SwapRoute {
            protocol: UniswapVersion::V2,
            path: vec!["TOKEN".to_string(), "WETH".to_string(), "USDC".to_string()],
            fee_tier: None,
        };

        assert_eq!(route.path.len(), 3);
    }

    #[test]
    fn test_swap_route_v3_fee_tiers() {
        // Test all common V3 fee tiers
        for fee in [100, 500, 3000, 10000] {
            let route = SwapRoute {
                protocol: UniswapVersion::V3,
                path: vec!["A".to_string(), "B".to_string()],
                fee_tier: Some(fee),
            };
            assert_eq!(route.fee_tier, Some(fee));
        }
    }

    // ============================================================================
    // Deadline Tests
    // ============================================================================

    #[test]
    fn test_deadline_20_minutes() {
        let now = current_timestamp();
        let deadline = now + 1200; // 20 minutes

        assert_eq!(deadline - now, 1200);
        assert!(deadline > now);
    }

    #[test]
    fn test_deadline_custom() {
        let custom_deadline = 1800000000u64;
        let params = SwapParams {
            from_token: Address::ZERO,
            to_token: Address::ZERO,
            amount_in: U256::ZERO,
            slippage_tolerance: Decimal::ONE,
            deadline: Some(custom_deadline),
        };

        assert_eq!(params.deadline, Some(custom_deadline));
    }
}
