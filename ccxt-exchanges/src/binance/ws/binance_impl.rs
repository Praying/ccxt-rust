// This file is included in mod.rs and contains Binance impl methods
// It's separated to keep mod.rs under 800 lines

impl Binance {
    /// Subscribes to the ticker stream for a unified symbol
    pub async fn subscribe_ticker(&self, symbol: &str) -> Result<()> {
        let ws = self.connection_manager.get_public_connection().await?;
        let binance_symbol = symbol.replace('/', "").to_lowercase();
        ws.subscribe_ticker(&binance_symbol).await
    }

    /// Subscribes to the trade stream for a unified symbol
    pub async fn subscribe_trades(&self, symbol: &str) -> Result<()> {
        let ws = self.connection_manager.get_public_connection().await?;
        let binance_symbol = symbol.replace('/', "").to_lowercase();
        ws.subscribe_trades(&binance_symbol).await
    }

    /// Subscribes to the order book stream for a unified symbol
    pub async fn subscribe_orderbook(&self, symbol: &str, levels: Option<u32>) -> Result<()> {
        let ws = self.connection_manager.get_public_connection().await?;
        let binance_symbol = symbol.replace('/', "").to_lowercase();
        let depth_levels = levels.unwrap_or(20);
        ws.subscribe_orderbook(&binance_symbol, depth_levels, "1000ms")
            .await
    }

    /// Subscribes to the candlestick stream for a unified symbol
    pub async fn subscribe_kline(&self, symbol: &str, interval: &str) -> Result<()> {
        let ws = self.connection_manager.get_public_connection().await?;
        let binance_symbol = symbol.replace('/', "").to_lowercase();
        ws.subscribe_kline(&binance_symbol, interval).await
    }

    /// Watches a ticker stream for a single unified symbol
    pub async fn watch_ticker(
        &self,
        symbol: &str,
        params: Option<HashMap<String, Value>>,
    ) -> Result<Ticker> {
        self.load_markets(false).await?;
        let market = self.base.market(symbol).await?;
        let binance_symbol = market.id.to_lowercase();

        let channel_name = if let Some(p) = &params {
            p.get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("ticker")
        } else {
            "ticker"
        };

        let ws = self.connection_manager.get_public_connection().await?;
        ws.watch_ticker_internal(&binance_symbol, channel_name)
            .await
    }

    /// Watches ticker streams for multiple unified symbols
    pub async fn watch_tickers(
        &self,
        symbols: Option<Vec<String>>,
        params: Option<HashMap<String, Value>>,
    ) -> Result<HashMap<String, Ticker>> {
        self.load_markets(false).await?;

        let channel_name = if let Some(p) = &params {
            p.get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("ticker")
        } else {
            "ticker"
        };

        if channel_name == "bookTicker" {
            return Err(Error::invalid_request(
                "To subscribe for bids-asks, use watch_bids_asks() method instead",
            ));
        }

        let binance_symbols = if let Some(syms) = symbols {
            let mut result = Vec::new();
            for symbol in syms {
                let market = self.base.market(&symbol).await?;
                result.push(market.id.to_lowercase());
            }
            Some(result)
        } else {
            None
        };

        let ws = self.connection_manager.get_public_connection().await?;
        ws.watch_tickers_internal(binance_symbols, channel_name)
            .await
    }

    /// Watches the mark price stream for a futures market
    pub async fn watch_mark_price(
        &self,
        symbol: &str,
        params: Option<HashMap<String, Value>>,
    ) -> Result<Ticker> {
        self.load_markets(false).await?;

        let market = self.base.market(symbol).await?;
        if market.market_type != MarketType::Swap && market.market_type != MarketType::Futures {
            return Err(Error::invalid_request(format!(
                "watch_mark_price() does not support {} markets",
                market.market_type
            )));
        }

        let binance_symbol = market.id.to_lowercase();

        let use_1s_freq = if let Some(p) = &params {
            p.get("use1sFreq")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true)
        } else {
            true
        };

        let channel_name = if use_1s_freq {
            "markPrice@1s"
        } else {
            "markPrice"
        };

        let ws = self.connection_manager.get_public_connection().await?;
        ws.watch_ticker_internal(&binance_symbol, channel_name)
            .await
    }

    /// Watches an order book stream for a unified symbol
    pub async fn watch_order_book(
        &self,
        symbol: &str,
        limit: Option<i64>,
        params: Option<HashMap<String, Value>>,
    ) -> Result<OrderBook> {
        self.load_markets(false).await?;

        let market = self.base.market(symbol).await?;
        let binance_symbol = market.id.to_lowercase();

        let is_futures =
            market.market_type == MarketType::Swap || market.market_type == MarketType::Futures;

        let update_speed = if let Some(p) = &params {
            p.get("speed")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(100) as i32
        } else {
            100
        };

        if update_speed != 100 && update_speed != 1000 {
            return Err(Error::invalid_request(
                "Update speed must be 100 or 1000 milliseconds",
            ));
        }

        let ws = self.connection_manager.get_public_connection().await?;
        ws.watch_orderbook_internal(self, &binance_symbol, limit, update_speed, is_futures)
            .await
    }

    /// Watches order books for multiple symbols
    pub async fn watch_order_books(
        &self,
        symbols: Vec<String>,
        limit: Option<i64>,
        params: Option<HashMap<String, Value>>,
    ) -> Result<HashMap<String, OrderBook>> {
        if symbols.is_empty() {
            return Err(Error::invalid_request("Symbols list cannot be empty"));
        }

        if symbols.len() > 200 {
            return Err(Error::invalid_request(
                "Binance supports max 200 symbols per connection",
            ));
        }

        self.load_markets(false).await?;

        let mut binance_symbols = Vec::new();
        let mut is_futures = false;

        for symbol in &symbols {
            let market = self.base.market(symbol).await?;
            binance_symbols.push(market.id.to_lowercase());

            let current_is_futures =
                market.market_type == MarketType::Swap || market.market_type == MarketType::Futures;
            if !binance_symbols.is_empty() && current_is_futures != is_futures {
                return Err(Error::invalid_request(
                    "Cannot mix spot and futures markets in watch_order_books",
                ));
            }
            is_futures = current_is_futures;
        }

        let update_speed = if let Some(p) = &params {
            p.get("speed")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(100) as i32
        } else {
            100
        };

        let ws = self.connection_manager.get_public_connection().await?;
        ws.watch_orderbooks_internal(self, binance_symbols, limit, update_speed, is_futures)
            .await
    }

    /// Watches mark prices for multiple futures symbols
    pub async fn watch_mark_prices(
        &self,
        symbols: Option<Vec<String>>,
        params: Option<HashMap<String, Value>>,
    ) -> Result<HashMap<String, Ticker>> {
        self.load_markets(false).await?;

        let use_1s_freq = if let Some(p) = &params {
            p.get("use1sFreq")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true)
        } else {
            true
        };

        let channel_name = if use_1s_freq {
            "markPrice@1s"
        } else {
            "markPrice"
        };

        let binance_symbols = if let Some(syms) = symbols {
            let mut result = Vec::new();
            for symbol in syms {
                let market = self.base.market(&symbol).await?;
                if market.market_type != MarketType::Swap
                    && market.market_type != MarketType::Futures
                {
                    return Err(Error::invalid_request(format!(
                        "watch_mark_prices() does not support {} markets",
                        market.market_type
                    )));
                }
                result.push(market.id.to_lowercase());
            }
            Some(result)
        } else {
            None
        };

        let ws = self.connection_manager.get_public_connection().await?;
        ws.watch_tickers_internal(binance_symbols, channel_name)
            .await
    }

    /// Streams trade data for a unified symbol
    pub async fn watch_trades(
        &self,
        symbol: &str,
        since: Option<i64>,
        limit: Option<usize>,
    ) -> Result<Vec<Trade>> {
        self.base.load_markets(false).await?;

        let market = self.base.market(symbol).await?;
        let binance_symbol = market.id.to_lowercase();

        let ws = self.connection_manager.get_public_connection().await?;

        ws.watch_trades_internal(symbol, &binance_symbol, since, limit, Some(&market))
            .await
    }

    /// Streams OHLCV data for a unified symbol
    pub async fn watch_ohlcv(
        &self,
        symbol: &str,
        timeframe: &str,
        since: Option<i64>,
        limit: Option<usize>,
    ) -> Result<Vec<OHLCV>> {
        self.base.load_markets(false).await?;

        let market = self.base.market(symbol).await?;
        let binance_symbol = market.id.to_lowercase();

        let ws = self.connection_manager.get_public_connection().await?;

        ws.watch_ohlcv_internal(symbol, &binance_symbol, timeframe, since, limit)
            .await
    }

    /// Streams the best bid/ask data for a unified symbol
    pub async fn watch_bids_asks(&self, symbol: &str) -> Result<BidAsk> {
        self.base.load_markets(false).await?;

        let market = self.base.market(symbol).await?;
        let binance_symbol = market.id.to_lowercase();

        let ws = self.connection_manager.get_public_connection().await?;

        ws.watch_bids_asks_internal(symbol, &binance_symbol).await
    }

    /// Streams account balance changes (private user data stream)
    pub async fn watch_balance(
        self: Arc<Self>,
        params: Option<HashMap<String, Value>>,
    ) -> Result<Balance> {
        self.base.load_markets(false).await?;

        let account_type = if let Some(p) = &params {
            p.get("type")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_else(|| self.options.default_type.as_str())
        } else {
            self.options.default_type.as_str()
        };

        let fetch_snapshot = if let Some(p) = &params {
            p.get("fetchBalanceSnapshot")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
        } else {
            false
        };

        let await_snapshot = if let Some(p) = &params {
            p.get("awaitBalanceSnapshot")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true)
        } else {
            true
        };

        let ws = self
            .connection_manager
            .get_private_connection(&self)
            .await?;

        if fetch_snapshot {
            let account_type_enum = account_type.parse::<ccxt_core::types::AccountType>().ok();
            let snapshot = self.fetch_balance(account_type_enum).await?;

            let mut balances = ws.balances.write().await;
            balances.insert(account_type.to_string(), snapshot.clone());

            if !await_snapshot {
                return Ok(snapshot);
            }
        }

        ws.watch_balance_internal(account_type).await
    }

    /// Watches authenticated order updates via the user data stream
    pub async fn watch_orders(
        self: Arc<Self>,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<usize>,
        _params: Option<HashMap<String, Value>>,
    ) -> Result<Vec<Order>> {
        self.base.load_markets(false).await?;

        let ws = self
            .connection_manager
            .get_private_connection(&self)
            .await?;
        ws.watch_orders_internal(symbol, since, limit).await
    }

    /// Watches authenticated user trade updates
    pub async fn watch_my_trades(
        self: Arc<Self>,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<usize>,
        _params: Option<HashMap<String, Value>>,
    ) -> Result<Vec<Trade>> {
        let ws = self
            .connection_manager
            .get_private_connection(&self)
            .await?;
        ws.watch_my_trades_internal(symbol, since, limit).await
    }

    /// Watches authenticated futures position updates
    pub async fn watch_positions(
        self: Arc<Self>,
        symbols: Option<Vec<String>>,
        since: Option<i64>,
        limit: Option<usize>,
        _params: Option<HashMap<String, Value>>,
    ) -> Result<Vec<Position>> {
        let ws = self
            .connection_manager
            .get_private_connection(&self)
            .await?;
        ws.watch_positions_internal(symbols, since, limit).await
    }
}
