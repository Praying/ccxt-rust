//! Trade type definitions

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::financial::{Amount, Cost, Price};
use super::order::{OrderSide, OrderType};
use super::{Fee, Symbol, Timestamp};

/// Trade data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    /// Trade ID
    pub id: Option<String>,

    /// Order ID that this trade belongs to
    pub order: Option<String>,

    /// Exchange symbol
    pub symbol: Symbol,

    /// Trade type (limit, market, etc.)
    #[serde(rename = "type")]
    pub trade_type: Option<OrderType>,

    /// Trade side (buy or sell)
    pub side: OrderSide,

    /// Whether this is a maker or taker trade
    #[allow(non_snake_case)]
    pub taker_or_maker: Option<TakerOrMaker>,

    /// Trade price
    pub price: Price,

    /// Trade amount
    pub amount: Amount,

    /// Trade cost (price * amount)
    pub cost: Option<Cost>,

    /// Trade fee
    pub fee: Option<Fee>,

    /// Timestamp in milliseconds
    pub timestamp: Timestamp,

    /// ISO8601 datetime string
    pub datetime: Option<String>,

    /// Raw exchange info
    #[serde(flatten)]
    pub info: HashMap<String, serde_json::Value>,
}

/// Whether a trade is maker or taker
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TakerOrMaker {
    /// Maker (provides liquidity)
    Maker,
    /// Taker (takes liquidity)
    Taker,
}

impl Trade {
    /// Creates a new trade with required fields.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol
    /// * `side` - Trade side (buy or sell)
    /// * `price` - Trade execution price
    /// * `amount` - Trade amount
    /// * `timestamp` - Trade timestamp in milliseconds
    pub fn new(
        symbol: Symbol,
        side: OrderSide,
        price: Price,
        amount: Amount,
        timestamp: Timestamp,
    ) -> Self {
        let cost = price * amount;

        Self {
            id: None,
            order: None,
            symbol,
            trade_type: None,
            side,
            taker_or_maker: None,
            price,
            amount,
            cost: Some(cost),
            fee: None,
            timestamp,
            datetime: None,
            info: HashMap::new(),
        }
    }

    /// Calculates and sets cost if not already set.
    pub fn calculate_cost(&mut self) {
        if self.cost.is_none() {
            self.cost = Some(self.price * self.amount);
        }
    }

    /// Returns `true` if this is a buy trade.
    pub fn is_buy(&self) -> bool {
        matches!(self.side, OrderSide::Buy)
    }

    /// Returns `true` if this is a sell trade.
    pub fn is_sell(&self) -> bool {
        matches!(self.side, OrderSide::Sell)
    }

    /// Returns `true` if this is a maker trade.
    pub fn is_maker(&self) -> bool {
        matches!(self.taker_or_maker, Some(TakerOrMaker::Maker))
    }

    /// Returns `true` if this is a taker trade.
    pub fn is_taker(&self) -> bool {
        matches!(self.taker_or_maker, Some(TakerOrMaker::Taker))
    }

    /// Returns the fee cost in quote currency if available.
    pub fn fee_cost(&self) -> Option<Decimal> {
        self.fee.as_ref().map(|f| f.cost)
    }
}

/// Aggregated trade data structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggTrade {
    /// Aggregated trade ID
    #[serde(rename = "a")]
    pub agg_id: i64,

    /// Trade price
    #[serde(rename = "p")]
    pub price: Decimal,

    /// Trade quantity
    #[serde(rename = "q")]
    pub quantity: Decimal,

    /// First trade ID in aggregation
    #[serde(rename = "f")]
    pub first_trade_id: i64,

    /// Last trade ID in aggregation
    #[serde(rename = "l")]
    pub last_trade_id: i64,

    /// Trade timestamp in milliseconds
    #[serde(rename = "T")]
    pub timestamp: i64,

    /// Whether buyer is the maker
    #[serde(rename = "m")]
    pub is_buyer_maker: bool,

    /// Whether trade was at best price match
    #[serde(rename = "M")]
    pub is_best_match: Option<bool>,

    /// Trading pair symbol (added for convenience, not native to Binance API)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
}

impl AggTrade {
    /// Creates a new aggregated trade instance.
    ///
    /// # Arguments
    ///
    /// * `agg_id` - Aggregated trade ID
    /// * `price` - Trade price
    /// * `quantity` - Trade quantity
    /// * `first_trade_id` - First trade ID in aggregation
    /// * `last_trade_id` - Last trade ID in aggregation
    /// * `timestamp` - Trade timestamp in milliseconds
    /// * `is_buyer_maker` - Whether buyer is the maker
    pub fn new(
        agg_id: i64,
        price: Decimal,
        quantity: Decimal,
        first_trade_id: i64,
        last_trade_id: i64,
        timestamp: i64,
        is_buyer_maker: bool,
    ) -> Self {
        Self {
            agg_id,
            price,
            quantity,
            first_trade_id,
            last_trade_id,
            timestamp,
            is_buyer_maker,
            is_best_match: None,
            symbol: None,
        }
    }

    /// Calculates the total cost (price × quantity).
    pub fn cost(&self) -> Decimal {
        self.price * self.quantity
    }

    /// Returns the trade side from the taker's perspective.
    pub fn side(&self) -> OrderSide {
        if self.is_buyer_maker {
            // Buyer is maker, so seller is taker
            OrderSide::Sell
        } else {
            // Seller is maker, so buyer is taker
            OrderSide::Buy
        }
    }

    /// Returns the number of trades included in this aggregation.
    pub fn trade_count(&self) -> i64 {
        self.last_trade_id - self.first_trade_id + 1
    }

    /// Formats timestamp as ISO8601 datetime string.
    pub fn datetime(&self) -> String {
        chrono::DateTime::from_timestamp_millis(self.timestamp)
            .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_trade_creation() {
        let trade = Trade::new(
            "BTC/USDT".to_string(),
            OrderSide::Buy,
            Price::new(dec!(50000)),
            Amount::new(dec!(0.5)),
            1234567890,
        );

        assert_eq!(trade.symbol, "BTC/USDT");
        assert_eq!(trade.price, Price::new(dec!(50000)));
        assert_eq!(trade.amount, Amount::new(dec!(0.5)));
        assert_eq!(trade.cost, Some(Cost::new(dec!(25000))));
    }

    #[test]
    fn test_trade_side_checks() {
        let buy_trade = Trade::new(
            "BTC/USDT".to_string(),
            OrderSide::Buy,
            Price::new(dec!(50000)),
            Amount::new(dec!(0.5)),
            1234567890,
        );

        assert!(buy_trade.is_buy());
        assert!(!buy_trade.is_sell());

        let sell_trade = Trade::new(
            "BTC/USDT".to_string(),
            OrderSide::Sell,
            Price::new(dec!(50000)),
            Amount::new(dec!(0.5)),
            1234567890,
        );

        assert!(sell_trade.is_sell());
        assert!(!sell_trade.is_buy());
    }

    #[test]
    fn test_maker_taker_checks() {
        let mut trade = Trade::new(
            "BTC/USDT".to_string(),
            OrderSide::Buy,
            Price::new(dec!(50000)),
            Amount::new(dec!(0.5)),
            1234567890,
        );

        assert!(!trade.is_maker());
        assert!(!trade.is_taker());

        trade.taker_or_maker = Some(TakerOrMaker::Maker);
        assert!(trade.is_maker());
        assert!(!trade.is_taker());

        trade.taker_or_maker = Some(TakerOrMaker::Taker);
        assert!(!trade.is_maker());
        assert!(trade.is_taker());
    }
}
