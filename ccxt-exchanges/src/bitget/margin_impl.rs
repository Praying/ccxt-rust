//! Margin trait implementation for Bitget.
//!
//! Implements the `Margin` trait from `ccxt-core` for Bitget, providing
//! position management, leverage configuration, margin mode, and funding rate operations.

use async_trait::async_trait;
use ccxt_core::{
    Result,
    capability::ExchangeCapabilities,
    traits::{Margin, PublicExchange},
    types::{
        FundingRate, FundingRateHistory, Position, Timeframe,
        params::{LeverageParams, MarginMode},
    },
};

use super::Bitget;

// ============================================================================
// PublicExchange Implementation
// ============================================================================

impl PublicExchange for Bitget {
    fn id(&self) -> &'static str {
        "bitget"
    }

    fn name(&self) -> &'static str {
        "Bitget"
    }

    fn version(&self) -> &'static str {
        "v2"
    }

    fn certified(&self) -> bool {
        false
    }

    fn capabilities(&self) -> ExchangeCapabilities {
        use ccxt_core::exchange::Capability;

        ExchangeCapabilities::builder()
            // Market Data
            .market_data()
            .without_capability(Capability::FetchCurrencies)
            .without_capability(Capability::FetchStatus)
            .without_capability(Capability::FetchTime)
            // Trading
            .trading()
            .without_capability(Capability::CancelAllOrders)
            .without_capability(Capability::EditOrder)
            .without_capability(Capability::FetchOrders)
            .without_capability(Capability::FetchCanceledOrders)
            // Account
            .capability(Capability::FetchBalance)
            .capability(Capability::FetchMyTrades)
            // Margin / Futures
            .capability(Capability::FetchPositions)
            .capability(Capability::SetLeverage)
            .capability(Capability::SetMarginMode)
            .capability(Capability::FetchFundingRate)
            .capability(Capability::FetchFundingRates)
            // WebSocket
            .capability(Capability::Websocket)
            .capability(Capability::WatchTicker)
            .capability(Capability::WatchOrderBook)
            .capability(Capability::WatchTrades)
            .capability(Capability::WatchBalance)
            .capability(Capability::WatchOrders)
            .capability(Capability::WatchMyTrades)
            .build()
    }

    fn timeframes(&self) -> Vec<Timeframe> {
        vec![
            Timeframe::M1,
            Timeframe::M5,
            Timeframe::M15,
            Timeframe::M30,
            Timeframe::H1,
            Timeframe::H4,
            Timeframe::H6,
            Timeframe::H12,
            Timeframe::D1,
            Timeframe::D3,
            Timeframe::W1,
            Timeframe::Mon1,
        ]
    }

    fn rate_limit(&self) -> u32 {
        20
    }

    fn has_websocket(&self) -> bool {
        true
    }
}

// ============================================================================
// Margin Trait Implementation
// ============================================================================

#[async_trait]
impl Margin for Bitget {
    async fn fetch_positions_for(&self, symbols: &[&str]) -> Result<Vec<Position>> {
        self.fetch_positions_impl(symbols).await
    }

    async fn fetch_position(&self, symbol: &str) -> Result<Position> {
        self.fetch_position_impl(symbol).await
    }

    async fn set_leverage_with_params(&self, params: LeverageParams) -> Result<()> {
        let margin_mode = params.margin_mode.map(|m| match m {
            MarginMode::Isolated => "isolated",
            MarginMode::Cross => "cross",
        });
        self.set_leverage_impl(&params.symbol, params.leverage, margin_mode)
            .await
    }

    async fn get_leverage(&self, symbol: &str) -> Result<u32> {
        self.get_leverage_impl(symbol).await
    }

    async fn set_margin_mode(&self, symbol: &str, mode: MarginMode) -> Result<()> {
        let mode_str = match mode {
            MarginMode::Isolated => "isolated",
            MarginMode::Cross => "cross",
        };
        self.set_margin_mode_impl(symbol, mode_str).await
    }

    async fn fetch_funding_rate(&self, symbol: &str) -> Result<FundingRate> {
        self.fetch_funding_rate_impl(symbol).await
    }

    async fn fetch_funding_rates(&self, symbols: &[&str]) -> Result<Vec<FundingRate>> {
        self.fetch_funding_rates_impl(symbols).await
    }

    async fn fetch_funding_rate_history(
        &self,
        symbol: &str,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<FundingRateHistory>> {
        self.fetch_funding_rate_history_impl(symbol, since, limit)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccxt_core::ExchangeConfig;

    #[test]
    fn test_bitget_public_exchange_impl() {
        let bitget = Bitget::new(ExchangeConfig::default()).unwrap();
        let pe: &dyn PublicExchange = &bitget;

        assert_eq!(pe.id(), "bitget");
        assert_eq!(pe.name(), "Bitget");
        assert_eq!(pe.version(), "v2");
        assert!(!pe.certified());
        assert!(pe.has_websocket());
        assert_eq!(pe.rate_limit(), 20);
    }

    #[test]
    fn test_bitget_capabilities_include_margin() {
        let bitget = Bitget::new(ExchangeConfig::default()).unwrap();
        let caps = PublicExchange::capabilities(&bitget);

        assert!(caps.has("fetchPositions"));
        assert!(caps.has("setLeverage"));
        assert!(caps.has("setMarginMode"));
        assert!(caps.has("fetchFundingRate"));
        assert!(caps.has("fetchFundingRates"));
    }

    #[test]
    fn test_bitget_margin_trait_object_safety() {
        let bitget = Bitget::new(ExchangeConfig::default()).unwrap();
        let _margin: Box<dyn Margin> = Box::new(bitget);
    }
}
