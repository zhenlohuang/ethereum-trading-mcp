//! Uniswap V3 contract bindings.

use alloy::sol;

// Re-export Uniswap V3 addresses from constants module.
pub use crate::ethereum::constants::{UNISWAP_V3_FACTORY, UNISWAP_V3_QUOTER, UNISWAP_V3_ROUTER};

/// Common fee tiers in Uniswap V3 (in basis points * 100).
pub mod fee_tiers {
    /// 0.01% fee tier.
    pub const FEE_LOWEST: u32 = 100;
    /// 0.05% fee tier.
    pub const FEE_LOW: u32 = 500;
    /// 0.30% fee tier.
    pub const FEE_MEDIUM: u32 = 3000;
    /// 1.00% fee tier.
    pub const FEE_HIGH: u32 = 10000;

    /// All available fee tiers.
    pub const ALL_FEES: [u32; 4] = [FEE_LOWEST, FEE_LOW, FEE_MEDIUM, FEE_HIGH];
}

// Uniswap V3 SwapRouter interface
sol! {
    #[sol(rpc)]
    interface ISwapRouter {
        struct ExactInputSingleParams {
            address tokenIn;
            address tokenOut;
            uint24 fee;
            address recipient;
            uint256 deadline;
            uint256 amountIn;
            uint256 amountOutMinimum;
            uint160 sqrtPriceLimitX96;
        }

        struct ExactInputParams {
            bytes path;
            address recipient;
            uint256 deadline;
            uint256 amountIn;
            uint256 amountOutMinimum;
        }

        struct ExactOutputSingleParams {
            address tokenIn;
            address tokenOut;
            uint24 fee;
            address recipient;
            uint256 deadline;
            uint256 amountOut;
            uint256 amountInMaximum;
            uint160 sqrtPriceLimitX96;
        }

        function exactInputSingle(ExactInputSingleParams calldata params) external payable returns (uint256 amountOut);
        function exactInput(ExactInputParams calldata params) external payable returns (uint256 amountOut);
        function exactOutputSingle(ExactOutputSingleParams calldata params) external payable returns (uint256 amountIn);
    }
}

// Uniswap V3 Factory interface
sol! {
    #[sol(rpc)]
    interface IUniswapV3Factory {
        function getPool(address tokenA, address tokenB, uint24 fee) external view returns (address pool);
    }
}

// Uniswap V3 Pool interface
sol! {
    #[sol(rpc)]
    interface IUniswapV3Pool {
        function token0() external view returns (address);
        function token1() external view returns (address);
        function fee() external view returns (uint24);
        function liquidity() external view returns (uint128);
        function slot0() external view returns (
            uint160 sqrtPriceX96,
            int24 tick,
            uint16 observationIndex,
            uint16 observationCardinality,
            uint16 observationCardinalityNext,
            uint8 feeProtocol,
            bool unlocked
        );
    }
}

// Uniswap V3 Quoter V2 interface
sol! {
    #[sol(rpc)]
    interface IQuoterV2 {
        struct QuoteExactInputSingleParams {
            address tokenIn;
            address tokenOut;
            uint256 amountIn;
            uint24 fee;
            uint160 sqrtPriceLimitX96;
        }

        function quoteExactInputSingle(QuoteExactInputSingleParams memory params)
            external
            returns (
                uint256 amountOut,
                uint160 sqrtPriceX96After,
                uint32 initializedTicksCrossed,
                uint256 gasEstimate
            );
    }
}
