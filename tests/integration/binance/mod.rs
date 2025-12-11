//! Binance交易所集成测试模块

pub mod test_public_api;
pub mod test_transfer;
pub mod test_margin_borrow;
pub mod test_ledger;
pub mod test_futures_transfer;
pub mod test_oco_orders;
pub mod test_market_data;
pub mod test_ohlcv_enhanced;
pub mod test_mock_ticker;

// 测试可以在不同模块中访问的公共常量
pub const TEST_SYMBOL: &str = "BTC/USDT";
pub const TEST_SYMBOLS: &[&str] = &["BTC/USDT", "ETH/USDT", "BNB/USDT"];