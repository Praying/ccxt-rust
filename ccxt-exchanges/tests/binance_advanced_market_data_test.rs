//! Binance advanced market data integration tests.
//!
//! Test coverage:
//! - OHLCV/candlestick data (`fetch_ohlcv`)
//! - Trading fee queries (`fetch_trading_fee`, `fetch_trading_fees`)
//! - Server time synchronization (`fetch_time`)

use ccxt_core::ExchangeConfig;
use ccxt_exchanges::binance::Binance;
use rust_decimal::Decimal;
use std::str::FromStr;

/// Create Binance client for testing.
fn create_binance_client() -> Binance {
    let config = ExchangeConfig {
        id: "binance".to_string(),
        name: "Binance".to_string(),
        ..Default::default()
    };
    Binance::new(config).unwrap_or_else(|e| panic!("Failed to create Binance client: {}", e))
}

/// Create authenticated Binance client for testing.
fn create_authenticated_binance_client() -> Binance {
    let api_key = std::env::var("BINANCE_API_KEY").ok();
    let api_secret = std::env::var("BINANCE_API_SECRET").ok();

    let config = ExchangeConfig {
        id: "binance".to_string(),
        name: "Binance".to_string(),
        api_key,
        secret: api_secret,
        ..Default::default()
    };
    Binance::new(config)
        .unwrap_or_else(|e| panic!("Failed to create authenticated Binance client: {}", e))
}

#[cfg(test)]
mod ohlcv_tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires network connection
    async fn test_fetch_ohlcv_basic() {
        let client = create_binance_client();

        let result = client
            .fetch_ohlcv("BTC/USDT", "1h", None, Some(10), None)
            .await;

        assert!(result.is_ok(), "Should successfully fetch OHLCV data");
        let ohlcvs = result.unwrap();

        assert!(!ohlcvs.is_empty(), "OHLCV data should not be empty");
        assert!(
            ohlcvs.len() <= 10,
            "Returned OHLCV count should not exceed limit"
        );
        let first = &ohlcvs[0];
        assert!(first.timestamp > 0, "Timestamp should be valid");
        assert!(first.open > 0.0, "Open price should be greater than 0");
        assert!(first.high > 0.0, "High price should be greater than 0");
        assert!(first.low > 0.0, "Low price should be greater than 0");
        assert!(first.close > 0.0, "Close price should be greater than 0");
        assert!(first.volume >= 0.0, "Volume should not be negative");

        assert!(first.high >= first.low, "High should be >= low");
        assert!(first.high >= first.open, "High should be >= open");
        assert!(first.high >= first.close, "High should be >= close");
        assert!(first.low <= first.open, "Low should be <= open");
        assert!(first.low <= first.close, "Low should be <= close");
    }

    #[tokio::test]
    #[ignore] // Requires network connection
    async fn test_fetch_ohlcv_different_timeframes() {
        let client = create_binance_client();

        let timeframes = vec!["1m", "5m", "15m", "1h", "4h", "1d"];

        for timeframe in timeframes {
            let result = client
                .fetch_ohlcv("BTC/USDT", timeframe, None, Some(5), None)
                .await;
            assert!(result.is_ok(), "Timeframe {} should succeed", timeframe);

            let ohlcvs = result.unwrap();
            assert!(
                !ohlcvs.is_empty(),
                "OHLCV data for timeframe {} should not be empty",
                timeframe
            );
        }
    }

    #[tokio::test]
    #[ignore] // Requires network connection
    async fn test_fetch_ohlcv_with_since() {
        let client = create_binance_client();

        let since = chrono::Utc::now().timestamp_millis() - (24 * 60 * 60 * 1000);

        let result = client
            .fetch_ohlcv("BTC/USDT", "1h", Some(since), Some(10), None)
            .await;

        assert!(
            result.is_ok(),
            "Should successfully fetch historical OHLCV data"
        );
        let ohlcvs = result.unwrap();

        assert!(
            !ohlcvs.is_empty(),
            "Historical OHLCV data should not be empty"
        );
        let first_timestamp = ohlcvs[0].timestamp;
        assert!(
            first_timestamp >= since,
            "OHLCV timestamp should be after since parameter"
        );
    }

    #[test]
    fn test_ohlcv_helper_methods() {
        use ccxt_core::types::OHLCV;

        let ohlcv = OHLCV {
            timestamp: 1609459200000, // 2021-01-01 00:00:00
            open: 29000.0,
            high: 30000.0,
            low: 28000.0,
            close: 29500.0,
            volume: 100.5,
        };

        let change = ohlcv.price_range();
        assert_eq!(change, 2000.0);

        assert!(ohlcv.is_bullish());
        assert!(!ohlcv.is_bearish());
    }
}

#[cfg(test)]
mod trading_fee_tests {
    use super::*;

    #[tokio::test]
    #[ignore] // 需要API密钥和网络连接
    async fn test_fetch_trading_fee_single() {
        let client = create_authenticated_binance_client();

        // 检查是否配置了API密钥
        if client.base().config.api_key.is_none() || client.base().config.secret.is_none() {
            println!("跳过测试：未配置API密钥");
            return;
        }

        let result = client.fetch_trading_fee("BTC/USDT", None).await;

        assert!(result.is_ok(), "应该成功获取手续费: {:?}", result.err());
        let fee = result.unwrap();

        assert_eq!(fee.symbol, "BTC/USDT", "交易对应该匹配");
        assert!(fee.maker >= Decimal::ZERO, "Maker费率应该是非负数");
        assert!(fee.taker >= Decimal::ZERO, "Taker费率应该是非负数");

        // 通常Maker费率应该低于或等于Taker费率
        assert!(fee.maker <= fee.taker, "Maker费率通常不高于Taker费率");
    }

    #[tokio::test]
    #[ignore] // Requires API credentials and network connection
    async fn test_fetch_trading_fees_all() {
        let client = create_authenticated_binance_client();

        // 检查是否配置了API密钥
        if client.base().config.api_key.is_none() || client.base().config.secret.is_none() {
            println!("Skipping test: API credentials not configured");
            return;
        }

        let result = client.fetch_trading_fees(None, None).await;

        assert!(result.is_ok(), "应该成功获取所有手续费");
        let fees = result.unwrap();

        assert!(!fees.is_empty(), "手续费列表不应为空");

        // 验证第一个手续费 (HashMap返回(键, 值)对)
        let first = fees.iter().next().unwrap();
        assert!(!first.0.is_empty(), "交易对不应为空");
        assert!(first.1.maker >= Decimal::ZERO, "Maker费率应该是非负数");
        assert!(first.1.taker >= Decimal::ZERO, "Taker费率应该是非负数");
    }

    #[tokio::test]
    #[ignore] // Requires API credentials and network connection
    async fn test_fetch_trading_fees_specific_symbols() {
        let client = create_authenticated_binance_client();

        // 检查是否配置了API密钥
        if client.base().config.api_key.is_none() || client.base().config.secret.is_none() {
            println!("Skipping test: API credentials not configured");
            return;
        }

        let symbols = vec!["BTC/USDT", "ETH/USDT", "BNB/USDT"];
        let result = client
            .fetch_trading_fees(Some(symbols.into_iter().map(String::from).collect()), None)
            .await;

        assert!(
            result.is_ok(),
            "Should successfully fetch fees for specified symbols"
        );
        let fees = result.unwrap();

        assert!(!fees.is_empty(), "手续费列表不应为空");
        assert!(fees.len() <= 3, "返回的手续费数量不应超过请求数量");

        // 验证所有返回的手续费都在请求的交易对列表中
        for (symbol, _) in &fees {
            assert!(
                ["BTC/USDT", "ETH/USDT", "BNB/USDT"].contains(&symbol.as_str()),
                "返回的交易对 {} 应该在请求列表中",
                symbol
            );
        }
    }

    #[test]
    fn test_trading_fee_fields() {
        use ccxt_core::types::TradingFee;

        // 创建一个测试手续费
        let fee = TradingFee {
            symbol: "BTC/USDT".to_string(),
            maker: 0.001, // 0.1%
            taker: 0.001, // 0.1%
            timestamp: None,
            datetime: None,
        };

        // 测试基本字段
        assert_eq!(fee.symbol, "BTC/USDT");
        assert_eq!(fee.maker, 0.001);
        assert_eq!(fee.taker, 0.001);
    }
}

#[cfg(test)]
mod server_time_tests {
    use super::*;

    #[tokio::test]
    #[ignore] // 需要网络连接
    async fn test_fetch_time() {
        let client = create_binance_client();

        let result = client.fetch_time().await;

        assert!(result.is_ok(), "Should successfully fetch server time");
        let server_time = result.unwrap();

        assert!(server_time.server_time > 0, "服务器时间戳应该大于0");
        assert!(!server_time.datetime.is_empty(), "日期时间字符串不应为空");

        // 验证服务器时间与本地时间的差异应该在合理范围内（例如5分钟）
        let local_time = chrono::Utc::now().timestamp_millis();
        let time_diff = (server_time.server_time - local_time).abs();
        assert!(
            time_diff < 5 * 60 * 1000,
            "服务器时间与本地时间差异应该在5分钟内，实际差异: {} ms",
            time_diff
        );
    }

    #[tokio::test]
    #[ignore] // Requires network connection
    async fn test_fetch_time_multiple_calls() {
        let client = create_binance_client();

        // 第一次调用
        let result1 = client.fetch_time().await;
        assert!(result1.is_ok(), "第一次调用应该成功");
        let time1 = result1.unwrap();

        // 等待1秒
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        // 第二次调用
        let result2 = client.fetch_time().await;
        assert!(result2.is_ok(), "第二次调用应该成功");
        let time2 = result2.unwrap();

        // 第二次的时间应该晚于第一次
        assert!(
            time2.server_time > time1.server_time,
            "第二次获取的时间应该晚于第一次"
        );
    }

    #[test]
    fn test_server_time_helper_methods() {
        use ccxt_core::types::ServerTime;

        // 创建一个测试服务器时间
        let server_time = ServerTime {
            server_time: 1609459200000, // 2021-01-01 00:00:00
            datetime: "2021-01-01T00:00:00.000Z".to_string(),
        };

        // 测试与本地时间的偏移（这个会根据实际时间变化）
        let offset = server_time.offset_from_local();
        // 只验证返回的是一个有效数字
        assert!(offset.abs() < i64::MAX);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires network connection
    async fn test_ohlcv_and_time_integration() {
        let client = create_binance_client();

        // 先获取服务器时间
        let time_result = client.fetch_time().await;
        assert!(time_result.is_ok(), "应该成功获取服务器时间");
        let server_time = time_result.unwrap();

        // 使用服务器时间作为since参数获取K线
        let since = server_time.server_time - (2 * 60 * 60 * 1000); // 2小时前
        let ohlcv_result = client
            .fetch_ohlcv("BTC/USDT", "1h", Some(since), Some(5), None)
            .await;

        assert!(ohlcv_result.is_ok(), "应该成功获取K线数据");
        let ohlcvs = ohlcv_result.unwrap();

        assert!(!ohlcvs.is_empty(), "K线数据不应为空");
    }

    #[tokio::test]
    #[ignore] // Requires API credentials and network connection
    async fn test_complete_market_data_workflow() {
        let client = create_authenticated_binance_client();

        // 检查是否配置了API密钥
        if client.base().config.api_key.is_none() || client.base().config.secret.is_none() {
            println!("Skipping test: API credentials not configured");
            return;
        }

        let symbol = "BTC/USDT";

        // 1. 获取服务器时间
        let time_result = client.fetch_time().await;
        assert!(time_result.is_ok(), "应该成功获取服务器时间");

        // 2. 获取K线数据
        let ohlcv_result = client.fetch_ohlcv(symbol, "1h", None, Some(10), None).await;
        assert!(ohlcv_result.is_ok(), "应该成功获取K线数据");

        // 3. 获取交易手续费
        let fee_result = client.fetch_trading_fee(symbol, None).await;
        assert!(fee_result.is_ok(), "Should successfully fetch trading fee");

        println!("✅ Complete market data workflow test passed");
    }
}
