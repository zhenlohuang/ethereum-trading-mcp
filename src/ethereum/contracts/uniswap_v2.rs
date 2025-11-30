//! Uniswap V2 contract bindings.

use alloy::sol;

// Re-export Uniswap V2 addresses from constants module.
pub use crate::ethereum::constants::{UNISWAP_V2_FACTORY, UNISWAP_V2_ROUTER};

// Uniswap V2 Router interface
sol! {
    #[sol(rpc)]
    interface IUniswapV2Router02 {
        function factory() external pure returns (address);
        function WETH() external pure returns (address);

        function swapExactTokensForTokens(
            uint amountIn,
            uint amountOutMin,
            address[] calldata path,
            address to,
            uint deadline
        ) external returns (uint[] memory amounts);

        function swapTokensForExactTokens(
            uint amountOut,
            uint amountInMax,
            address[] calldata path,
            address to,
            uint deadline
        ) external returns (uint[] memory amounts);

        function swapExactETHForTokens(
            uint amountOutMin,
            address[] calldata path,
            address to,
            uint deadline
        ) external payable returns (uint[] memory amounts);

        function swapTokensForExactETH(
            uint amountOut,
            uint amountInMax,
            address[] calldata path,
            address to,
            uint deadline
        ) external returns (uint[] memory amounts);

        function swapExactTokensForETH(
            uint amountIn,
            uint amountOutMin,
            address[] calldata path,
            address to,
            uint deadline
        ) external returns (uint[] memory amounts);

        function swapETHForExactTokens(
            uint amountOut,
            address[] calldata path,
            address to,
            uint deadline
        ) external payable returns (uint[] memory amounts);

        function getAmountsOut(
            uint amountIn,
            address[] calldata path
        ) external view returns (uint[] memory amounts);

        function getAmountsIn(
            uint amountOut,
            address[] calldata path
        ) external view returns (uint[] memory amounts);
    }
}

// Uniswap V2 Factory interface
sol! {
    #[sol(rpc)]
    interface IUniswapV2Factory {
        function getPair(address tokenA, address tokenB) external view returns (address pair);
        function allPairs(uint) external view returns (address pair);
        function allPairsLength() external view returns (uint);
    }
}

// Uniswap V2 Pair interface
sol! {
    #[sol(rpc)]
    interface IUniswapV2Pair {
        function token0() external view returns (address);
        function token1() external view returns (address);
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
        function price0CumulativeLast() external view returns (uint);
        function price1CumulativeLast() external view returns (uint);
    }
}
