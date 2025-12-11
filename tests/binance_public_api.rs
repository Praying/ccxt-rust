//! Binance公开API集成测试
//!
//! 这个测试套件验证Binance交易所的公开API功能

// 声明集成测试模块路径
#[path = "integration/common/mod.rs"]
mod common;

// 直接包含binance测试模块
#[path = "integration/binance/test_public_api.rs"]
mod test_public_api;

#[path = "integration/binance/test_mock_ticker.rs"]
mod test_mock_ticker;

// 测试会自动运行，无需额外导出
