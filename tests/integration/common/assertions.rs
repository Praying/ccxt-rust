//! 集成测试自定义断言宏和函数
//!
//! 提供用于验证交易所API返回数据完整性和正确性的断言工具

// Allow clippy warnings for test helper code
#![allow(dead_code)]
#![allow(unused_imports)]

use ccxt_core::types::{Amount, Balance, OrderBook, OrderBookEntry, Price, Ticker, Trade};
use rust_decimal::prelude::*;
use std::collections::HashMap;

/// 验证Ticker数据的完整性和合理性
#[macro_export]
macro_rules! assert_valid_ticker {
    ($ticker:expr, $symbol:expr) => {{
        let ticker = &$ticker;

        // 基本字段验证
        assert_eq!(
            ticker.symbol.as_str(),
            $symbol,
            "Ticker symbol should match"
        );
        assert!(ticker.timestamp > 0, "Ticker timestamp should be positive");

        // 价格字段验证
        if let Some(last) = ticker.last {
            assert!(last > Price(Decimal::ZERO), "Last price should be positive");
        }
        assert!(
            ticker.bid.unwrap_or(Price(Decimal::ZERO)) >= Price(Decimal::ZERO),
            "Bid price should be non-negative"
        );
        assert!(
            ticker.ask.unwrap_or(Price(Decimal::ZERO)) >= Price(Decimal::ZERO),
            "Ask price should be non-negative"
        );

        // 价格合理性验证
        if let (Some(bid), Some(ask)) = (ticker.bid, ticker.ask) {
            assert!(
                bid > Price(Decimal::ZERO) && ask > Price(Decimal::ZERO),
                "Bid and ask should be positive"
            );
            assert!(
                bid <= ask,
                "Bid should not exceed ask: bid={}, ask={}",
                bid,
                ask
            );
        }

        // 24小时数据验证
        if let (Some(high), Some(last)) = (ticker.high, ticker.last) {
            assert!(high >= last, "High should be >= last price");
        }
        if let (Some(low), Some(last)) = (ticker.low, ticker.last) {
            assert!(low <= last, "Low should be <= last price");
        }

        // 成交量验证
        if let Some(volume) = ticker.base_volume {
            assert!(
                volume >= Amount(Decimal::ZERO),
                "Base volume should be non-negative"
            );
        }
        if let Some(volume) = ticker.quote_volume {
            assert!(
                volume >= Amount(Decimal::ZERO),
                "Quote volume should be non-negative"
            );
        }

        println!("✓ Ticker validation passed for {}", $symbol);
    }};
}

/// 验证OrderBook数据的完整性和合理性
#[macro_export]
macro_rules! assert_valid_orderbook {
    ($orderbook:expr, $symbol:expr) => {{
        let orderbook = &$orderbook;

        // 基本字段验证
        assert_eq!(orderbook.symbol, $symbol, "OrderBook symbol should match");
        assert!(
            orderbook.timestamp > 0,
            "OrderBook timestamp should be positive"
        );

        // Bids和Asks非空验证
        assert!(
            !orderbook.bids.is_empty(),
            "OrderBook bids should not be empty"
        );
        assert!(
            !orderbook.asks.is_empty(),
            "OrderBook asks should not be empty"
        );

        // 价格和数量验证
        for (i, bid) in orderbook.bids.iter().enumerate() {
            assert!(
                bid.price > Price(Decimal::ZERO),
                "Bid[{}] price should be positive",
                i
            );
            assert!(
                bid.amount > Amount(Decimal::ZERO),
                "Bid[{}] amount should be positive",
                i
            );
        }

        for (i, ask) in orderbook.asks.iter().enumerate() {
            assert!(
                ask.price > Price(Decimal::ZERO),
                "Ask[{}] price should be positive",
                i
            );
            assert!(
                ask.amount > Amount(Decimal::ZERO),
                "Ask[{}] amount should be positive",
                i
            );
        }

        // 价格顺序验证（bids降序，asks升序）
        for i in 1..orderbook.bids.len() {
            assert!(
                orderbook.bids[i - 1].price >= orderbook.bids[i].price,
                "Bids should be in descending order"
            );
        }

        for i in 1..orderbook.asks.len() {
            assert!(
                orderbook.asks[i - 1].price <= orderbook.asks[i].price,
                "Asks should be in ascending order"
            );
        }

        // 买卖价差验证
        let best_bid = orderbook.bids[0].price;
        let best_ask = orderbook.asks[0].price;
        assert!(
            best_bid < best_ask,
            "Best bid should be less than best ask: bid={}, ask={}",
            best_bid,
            best_ask
        );

        println!(
            "✓ OrderBook validation passed for {} (bids: {}, asks: {})",
            $symbol,
            orderbook.bids.len(),
            orderbook.asks.len()
        );
    }};

    ($orderbook:expr, $symbol:expr, $min_depth:expr) => {{
        assert_valid_orderbook!($orderbook, $symbol);

        assert!(
            $orderbook.bids.len() >= $min_depth,
            "OrderBook should have at least {} bids",
            $min_depth
        );
        assert!(
            $orderbook.asks.len() >= $min_depth,
            "OrderBook should have at least {} asks",
            $min_depth
        );

        println!("✓ OrderBook depth validation passed (min: {})", $min_depth);
    }};
}

/// 验证Trade数据的完整性和合理性
#[macro_export]
macro_rules! assert_valid_trade {
    ($trade:expr, $symbol:expr) => {{
        let trade = &$trade;

        // 基本字段验证
        if let Some(id) = &trade.id {
            assert!(!id.is_empty(), "Trade id should not be empty");
        }
        assert_eq!(trade.symbol, $symbol, "Trade symbol should match");
        assert!(trade.timestamp > 0, "Trade timestamp should be positive");

        // 价格和数量验证
        assert!(
            trade.price > Price(Decimal::ZERO),
            "Trade price should be positive"
        );
        assert!(
            trade.amount > Amount(Decimal::ZERO),
            "Trade amount should be positive"
        );

        // 成本验证（如果存在）
        if let Some(cost) = trade.cost {
            assert!(cost > Cost(Decimal::ZERO), "Trade cost should be positive");
            let price_dec: Decimal = trade.price.into();
            let amount_dec: Decimal = trade.amount.into();
            let cost_dec: Decimal = cost.into();
            let expected_cost = price_dec * amount_dec;
            let diff = (cost_dec - expected_cost).abs();
            let tolerance = Decimal::new(1, 2); // 0.01 = 1%
            assert!(
                diff / expected_cost < tolerance,
                "Trade cost should match price * amount (within 1%)"
            );
        }

        // 方向验证 - OrderSide是enum，验证字段存在即可
        let _ = &trade.side;

        println!(
            "✓ Trade validation passed: {} {} @ {}",
            trade.amount, trade.symbol, trade.price
        );
    }};
}

/// 验证Market数据的完整性和合理性
#[macro_export]
macro_rules! assert_valid_market {
    ($market:expr) => {{
        let market = &$market;

        // 基本字段验证
        assert!(!market.id.is_empty(), "Market id should not be empty");
        assert!(
            !market.symbol.to_string().is_empty(),
            "Market symbol should not be empty"
        );
        assert!(!market.base.is_empty(), "Market base should not be empty");
        assert!(!market.quote.is_empty(), "Market quote should not be empty");

        // Symbol格式验证（应为 BASE/QUOTE 格式）
        let symbol_str = market.symbol.to_string();
        assert!(
            symbol_str.contains('/'),
            "Market symbol should contain '/' separator"
        );

        // 活跃状态验证
        assert!(market.active, "Market should be active");

        // 精度验证 - precision字段是Option<Decimal>类型
        if let Some(amount_precision) = market.precision.amount {
            assert!(
                amount_precision >= Decimal::ZERO && amount_precision <= Decimal::from(18),
                "Amount precision should be between 0 and 18"
            );
        }
        if let Some(price_precision) = market.precision.price {
            assert!(
                price_precision >= Decimal::ZERO && price_precision <= Decimal::from(18),
                "Price precision should be between 0 and 18"
            );
        }

        // 限制验证 - limits.amount是Option<MinMax>，需要先展开
        if let Some(amount_limits) = &market.limits.amount {
            if let Some(min_amount) = amount_limits.min {
                assert!(min_amount > Decimal::ZERO, "Min amount should be positive");
            }
        }
        if let Some(cost_limits) = &market.limits.cost {
            if let Some(min_cost) = cost_limits.min {
                assert!(min_cost > Decimal::ZERO, "Min cost should be positive");
            }
        }

        println!("✓ Market validation passed: {}", market.symbol.to_string());
    }};
}

/// 验证Balance数据的完整性和合理性
#[macro_export]
macro_rules! assert_valid_balance {
    ($balance:expr) => {{
        let balance = &$balance;

        // 基本字段验证
        assert!(
            !balance.balances.is_empty(),
            "Balance should contain at least one currency"
        );

        // 验证每个币种余额
        for (currency, info) in &balance.balances {
            assert!(!currency.is_empty(), "Currency code should not be empty");
            assert!(
                info.free >= Decimal::ZERO,
                "Free balance should be non-negative for {}",
                currency
            );
            assert!(
                info.used >= Decimal::ZERO,
                "Used balance should be non-negative for {}",
                currency
            );
            assert!(
                info.total >= Decimal::ZERO,
                "Total balance should be non-negative for {}",
                currency
            );

            // 总额验证
            let expected_total = info.free + info.used;
            let diff = (info.total - expected_total).abs();
            let tolerance = Decimal::new(1, 6); // 0.000001
            assert!(
                diff < tolerance,
                "Total should equal free + used for {} (total={}, free={}, used={})",
                currency,
                info.total,
                info.free,
                info.used
            );
        }

        println!(
            "✓ Balance validation passed ({} currencies)",
            balance.balances.len()
        );
    }};
}

/// 断言两个浮点数近似相等（用于价格比较）
pub fn assert_approx_eq(a: f64, b: f64, tolerance: f64, msg: &str) {
    let diff = (a - b).abs();
    assert!(
        diff <= tolerance,
        "{}: expected {}, got {}, diff={} (tolerance={})",
        msg,
        a,
        b,
        diff,
        tolerance
    );
}

/// 验证时间戳在合理范围内（不能太旧或太新）
pub fn assert_reasonable_timestamp(timestamp: i64, max_age_seconds: i64) {
    let now = chrono::Utc::now().timestamp_millis();
    let age = (now - timestamp).abs();

    assert!(
        age <= max_age_seconds * 1000,
        "Timestamp is too old or in the future: age={}s (max={}s)",
        age / 1000,
        max_age_seconds
    );
}

/// 验证字符串不为空且符合预期格式
pub fn assert_non_empty_string(s: &str, field_name: &str) {
    assert!(!s.is_empty(), "{} should not be empty", field_name);
    assert!(
        !s.trim().is_empty(),
        "{} should not be whitespace only",
        field_name
    );
}

/// 创建测试用的Ticker数据
pub fn create_test_ticker(symbol: &str) -> Ticker {
    use rust_decimal_macros::dec;

    Ticker {
        symbol: symbol.to_string(),
        timestamp: chrono::Utc::now().timestamp_millis(),
        datetime: Some(chrono::Utc::now().to_rfc3339()),
        last: Some(dec!(100.0).into()),
        bid: Some(dec!(99.5).into()),
        ask: Some(dec!(100.5).into()),
        bid_volume: None,
        ask_volume: None,
        high: Some(dec!(105.0).into()),
        low: Some(dec!(95.0).into()),
        base_volume: Some(dec!(1000.0).into()),
        quote_volume: Some(dec!(100000.0).into()),
        open: Some(dec!(98.0).into()),
        close: Some(dec!(100.0).into()),
        previous_close: None,
        change: None,
        percentage: None,
        average: None,
        vwap: None,
        funding_rate: None,
        open_interest: None,
        index_price: None,
        mark_price: None,
        info: HashMap::new(),
    }
}

/// 创建测试用的Trade数据
pub fn create_test_trade(symbol: &str) -> Trade {
    use ccxt_core::types::order::OrderSide;
    use rust_decimal_macros::dec;

    Trade {
        id: Some("12345".to_string()),
        order: Some("order123".to_string()),
        symbol: symbol.to_string(),
        trade_type: None,
        side: OrderSide::Buy,
        taker_or_maker: None,
        price: dec!(100.0).into(),
        amount: dec!(1.0).into(),
        cost: Some(dec!(100.0).into()),
        fee: None,
        timestamp: chrono::Utc::now().timestamp_millis(),
        datetime: Some(chrono::Utc::now().to_rfc3339()),
        info: HashMap::new(),
    }
}

/// 创建测试用的OrderBook数据
pub fn create_test_orderbook(symbol: &str) -> OrderBook {
    use rust_decimal_macros::dec;
    use std::collections::{BTreeMap, VecDeque};

    OrderBook {
        symbol: symbol.to_string(),
        bids: vec![
            OrderBookEntry {
                price: dec!(99.5).into(),
                amount: dec!(10.0).into(),
            },
            OrderBookEntry {
                price: dec!(99.0).into(),
                amount: dec!(20.0).into(),
            },
        ],
        asks: vec![
            OrderBookEntry {
                price: dec!(100.5).into(),
                amount: dec!(10.0).into(),
            },
            OrderBookEntry {
                price: dec!(101.0).into(),
                amount: dec!(20.0).into(),
            },
        ],
        timestamp: chrono::Utc::now().timestamp_millis(),
        datetime: Some(chrono::Utc::now().to_rfc3339()),
        nonce: None,
        info: HashMap::new(),
        buffered_deltas: VecDeque::new(),
        bids_map: BTreeMap::new(),
        asks_map: BTreeMap::new(),
        is_synced: false,
        last_resync_time: 0,
        needs_resync: false,
    }
}

/// 创建测试用的Balance数据
pub fn create_test_balance() -> Balance {
    use ccxt_core::types::balance::BalanceEntry;
    use rust_decimal_macros::dec;

    let mut balances = HashMap::new();
    balances.insert(
        "BTC".to_string(),
        BalanceEntry {
            free: dec!(1.0),
            used: dec!(0.5),
            total: dec!(1.5),
        },
    );
    balances.insert(
        "USDT".to_string(),
        BalanceEntry {
            free: dec!(10000.0),
            used: dec!(5000.0),
            total: dec!(15000.0),
        },
    );

    Balance {
        balances,
        info: HashMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccxt_core::types::BalanceEntry;
    use rust_decimal_macros::dec;
    use std::collections::{BTreeMap, HashMap, VecDeque};

    #[test]
    fn test_assert_approx_eq() {
        assert_approx_eq(1.0, 1.0001, 0.001, "Should be approximately equal");
        assert_approx_eq(
            100.0,
            100.05,
            0.1,
            "Should be approximately equal with larger tolerance",
        );
    }

    #[test]
    #[should_panic(expected = "expected 1, got 2")]
    fn test_assert_approx_eq_failure() {
        assert_approx_eq(1.0, 2.0, 0.1, "Should fail");
    }

    #[test]
    fn test_assert_reasonable_timestamp() {
        let now = chrono::Utc::now().timestamp_millis();
        assert_reasonable_timestamp(now, 60); // 当前时间应该在合理范围内
        assert_reasonable_timestamp(now - 30000, 60); // 30秒前也在范围内
    }

    #[test]
    #[should_panic(expected = "too old")]
    fn test_assert_reasonable_timestamp_too_old() {
        let old = chrono::Utc::now().timestamp_millis() - 120000; // 2分钟前
        assert_reasonable_timestamp(old, 60); // 最大1分钟，应该失败
    }

    #[test]
    fn test_assert_non_empty_string() {
        assert_non_empty_string("BTCUSDT", "symbol");
        assert_non_empty_string("test", "field");
    }

    #[test]
    #[should_panic(expected = "should not be empty")]
    fn test_assert_non_empty_string_failure() {
        assert_non_empty_string("", "field");
    }

    #[test]
    fn test_ticker_validation_macro() {
        let ticker = Ticker {
            symbol: "BTC/USDT".to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            datetime: Some(chrono::Utc::now().to_rfc3339()),
            high: Some(dec!(50000.0).into()),
            low: Some(dec!(48000.0).into()),
            bid: Some(dec!(49000.0).into()),
            ask: Some(dec!(49100.0).into()),
            bid_volume: None,
            ask_volume: None,
            last: Some(dec!(49050.0).into()),
            close: Some(dec!(49050.0).into()),
            base_volume: Some(dec!(1000.0).into()),
            quote_volume: Some(dec!(49000000.0).into()),
            open: None,
            previous_close: None,
            change: None,
            percentage: None,
            average: None,
            vwap: None,
            funding_rate: None,
            open_interest: None,
            index_price: None,
            mark_price: None,
            info: HashMap::new(),
        };

        assert_valid_ticker!(ticker, "BTC/USDT");
    }

    #[test]
    fn test_orderbook_validation_macro() {
        let orderbook = OrderBook {
            symbol: "BTC/USDT".to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            datetime: Some(chrono::Utc::now().to_rfc3339()),
            bids: vec![
                OrderBookEntry {
                    price: dec!(49000.0).into(),
                    amount: dec!(1.0).into(),
                },
                OrderBookEntry {
                    price: dec!(48900.0).into(),
                    amount: dec!(2.0).into(),
                },
                OrderBookEntry {
                    price: dec!(48800.0).into(),
                    amount: dec!(3.0).into(),
                },
            ],
            asks: vec![
                OrderBookEntry {
                    price: dec!(49100.0).into(),
                    amount: dec!(1.0).into(),
                },
                OrderBookEntry {
                    price: dec!(49200.0).into(),
                    amount: dec!(2.0).into(),
                },
                OrderBookEntry {
                    price: dec!(49300.0).into(),
                    amount: dec!(3.0).into(),
                },
            ],
            nonce: Some(12345),
            info: HashMap::new(),
            buffered_deltas: VecDeque::new(),
            bids_map: BTreeMap::new(),
            asks_map: BTreeMap::new(),
            is_synced: false,
            last_resync_time: 0,
            needs_resync: false,
        };

        assert_valid_orderbook!(orderbook, "BTC/USDT");
        assert_valid_orderbook!(orderbook, "BTC/USDT", 3);
    }

    // 注意：这里移除了测试辅助函数的单元测试
    // 因为它们使用了不正确的数据结构定义
    // 真正的集成测试将在 test_public_api.rs 中进行
}
