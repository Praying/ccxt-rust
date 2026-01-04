// This file is included in mod.rs and contains Binance impl methods
// It's separated to keep mod.rs under 800 lines

impl Binance {
    /// Subscribes to the ticker stream for a unified symbol
    pub async fn subscribe_ticker(&self, symbol: &str) -> Result<()> {
        let ws = self.create_ws();
        ws.connect().await?;
        let binance_symbol = symbol.replace('/', "").to_lowercase();
        ws.subscribe_ticker(&binance_symbol).await
    }

    /// Subscribes to the trade stream for a unified symbol
    pub async fn subscribe_trades(&self, symbol: &str) -> Result<()> {
        let ws = self.create_ws();
        ws.connect().await?;
        let binance_symbol = symbol.replace('/', "").to_lowercase();
        ws.subscribe_trades(&binance_symbol).await
    }

    /// Subscribes to the order book stream for a unified symbol
    pub async fn subscribe_orderbook(&self, symbol: &str, levels: Option<u32>) -> Result<()> {
        let ws = self.create_ws();
        ws.connect().await?;
        let binance_symbol = symbol.replace('/', "").to_lowercase();
        let depth_levels = levels.unwrap_or(20);
        ws.subscribe_orderbook(&binance_symbol, depth_levels, "1000ms")
            .await
    }

    /// Subscribes to the candlestick stream for a unified symbol
    pub async fn subscribe_kline(&self, symbol: &str, interval: &str) -> Result<()> {
        let ws = self.create_ws();
        ws.connect().await?;
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

        let ws = self.create_ws();
        ws.connect().await?;
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

        let ws = self.create_ws();
        ws.connect().await?;
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

        let ws = self.create_ws();
        ws.connect().await?;
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

        let ws = self.create_ws();
        ws.connect().await?;
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

        let ws = self.create_ws();
        ws.connect().await?;
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

        let ws = self.create_ws();
        ws.connect().await?;
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
        const MAX_RETRIES: u32 = 50;
        const MAX_TRADES: usize = 1000;

        self.base.load_markets(false).await?;

        let market = self.base.market(symbol).await?;
        let binance_symbol = market.id.to_lowercase();

        let ws = self.create_ws();
        ws.connect().await?;

        ws.subscribe_trades(&binance_symbol).await?;

        let mut retries = 0;

        while retries < MAX_RETRIES {
            if let Some(msg) = ws.client.receive().await {
                if msg.get("result").is_some() || msg.get("id").is_some() {
                    continue;
                }

                if let Ok(trade) = parser::parse_ws_trade(&msg, Some(&market)) {
                    let mut trades_map = ws.trades.lock().await;
                    let trades = trades_map
                        .entry(symbol.to_string())
                        .or_insert_with(VecDeque::new);

                    if trades.len() >= MAX_TRADES {
                        trades.pop_front();
                    }
                    trades.push_back(trade);

                    let mut result: Vec<Trade> = trades.iter().cloned().collect();

                    if let Some(since_ts) = since {
                        result.retain(|t| t.timestamp >= since_ts);
                    }

                    if let Some(limit_size) = limit {
                        if result.len() > limit_size {
                            result = result.split_off(result.len() - limit_size);
                        }
                    }

                    return Ok(result);
                }
            }

            retries += 1;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Err(Error::network("Timeout waiting for trade data"))
    }

    /// Streams OHLCV data for a unified symbol
    pub async fn watch_ohlcv(
        &self,
        symbol: &str,
        timeframe: &str,
        since: Option<i64>,
        limit: Option<usize>,
    ) -> Result<Vec<OHLCV>> {
        const MAX_RETRIES: u32 = 50;
        const MAX_OHLCVS: usize = 1000;

        self.base.load_markets(false).await?;

        let market = self.base.market(symbol).await?;
        let binance_symbol = market.id.to_lowercase();

        let ws = self.create_ws();
        ws.connect().await?;

        ws.subscribe_kline(&binance_symbol, timeframe).await?;

        let mut retries = 0;

        while retries < MAX_RETRIES {
            if let Some(msg) = ws.client.receive().await {
                if msg.get("result").is_some() || msg.get("id").is_some() {
                    continue;
                }

                if let Ok(ohlcv) = parser::parse_ws_ohlcv(&msg) {
                    let cache_key = format!("{}:{}", symbol, timeframe);
                    let mut ohlcvs_map = ws.ohlcvs.lock().await;
                    let ohlcvs = ohlcvs_map.entry(cache_key).or_insert_with(VecDeque::new);

                    if ohlcvs.len() >= MAX_OHLCVS {
                        ohlcvs.pop_front();
                    }
                    ohlcvs.push_back(ohlcv);

                    let mut result: Vec<OHLCV> = ohlcvs.iter().cloned().collect();

                    if let Some(since_ts) = since {
                        result.retain(|o| o.timestamp >= since_ts);
                    }

                    if let Some(limit_size) = limit {
                        if result.len() > limit_size {
                            result = result.split_off(result.len() - limit_size);
                        }
                    }

                    return Ok(result);
                }
            }

            retries += 1;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Err(Error::network("Timeout waiting for OHLCV data"))
    }

    /// Streams the best bid/ask data for a unified symbol
    pub async fn watch_bids_asks(&self, symbol: &str) -> Result<BidAsk> {
        const MAX_RETRIES: u32 = 50;

        self.base.load_markets(false).await?;

        let market = self.base.market(symbol).await?;
        let binance_symbol = market.id.to_lowercase();

        let ws = self.create_ws();
        ws.connect().await?;

        let stream_name = format!("{}@bookTicker", binance_symbol);
        ws.client
            .subscribe(stream_name, Some(symbol.to_string()), None)
            .await?;

        let mut retries = 0;

        while retries < MAX_RETRIES {
            if let Some(msg) = ws.client.receive().await {
                if msg.get("result").is_some() || msg.get("id").is_some() {
                    continue;
                }

                if let Ok(bid_ask) = parser::parse_ws_bid_ask(&msg) {
                    let mut bids_asks_map = ws.bids_asks.lock().await;
                    bids_asks_map.insert(symbol.to_string(), bid_ask.clone());

                    return Ok(bid_ask);
                }
            }

            retries += 1;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Err(Error::network("Timeout waiting for BidAsk data"))
    }

    /// Streams account balance changes (private user data stream)
    pub async fn watch_balance(
        self: Arc<Self>,
        params: Option<HashMap<String, Value>>,
    ) -> Result<Balance> {
        const MAX_RETRIES: u32 = 100;

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

        let ws = self.create_authenticated_ws();
        ws.connect().await?;

        if fetch_snapshot {
            let account_type_enum = account_type.parse::<ccxt_core::types::AccountType>().ok();
            let snapshot = self.fetch_balance(account_type_enum).await?;

            let mut balances = ws.balances.write().await;
            balances.insert(account_type.to_string(), snapshot.clone());

            if !await_snapshot {
                return Ok(snapshot);
            }
        }

        let mut retries = 0;

        while retries < MAX_RETRIES {
            if let Some(msg) = ws.client.receive().await {
                if msg.get("result").is_some() || msg.get("id").is_some() {
                    continue;
                }

                if let Some(event_type) = msg.get("e").and_then(|e| e.as_str()) {
                    if matches!(
                        event_type,
                        "balanceUpdate" | "outboundAccountPosition" | "ACCOUNT_UPDATE"
                    ) {
                        if let Ok(()) = ws.handle_balance_message(&msg, account_type).await {
                            let balances = ws.balances.read().await;
                            if let Some(balance) = balances.get(account_type) {
                                return Ok(balance.clone());
                            }
                        }
                    }
                }
            }

            retries += 1;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Err(Error::network("Timeout waiting for balance data"))
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

        let ws = self.create_authenticated_ws();
        ws.connect().await?;

        loop {
            if let Some(msg) = ws.client.receive().await {
                if let Value::Object(data) = msg {
                    if let Some(event_type) = data.get("e").and_then(serde_json::Value::as_str) {
                        if event_type == "executionReport" {
                            let order = user_data::parse_ws_order(&data);

                            let mut orders = ws.orders.write().await;
                            let symbol_orders = orders
                                .entry(order.symbol.clone())
                                .or_insert_with(HashMap::new);
                            symbol_orders.insert(order.id.clone(), order.clone());
                            drop(orders);

                            if let Some(exec_type) =
                                data.get("x").and_then(serde_json::Value::as_str)
                            {
                                if exec_type == "TRADE" {
                                    if let Ok(trade) =
                                        BinanceWs::parse_ws_trade(&Value::Object(data.clone()))
                                    {
                                        let mut trades = ws.my_trades.write().await;
                                        let symbol_trades = trades
                                            .entry(trade.symbol.clone())
                                            .or_insert_with(VecDeque::new);

                                        symbol_trades.push_front(trade);
                                        if symbol_trades.len() > 1000 {
                                            symbol_trades.pop_back();
                                        }
                                    }
                                }
                            }

                            return self.filter_orders(&ws, symbol, since, limit).await;
                        }
                    }
                }
            } else {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }

    /// Filters cached orders by symbol, time range, and limit
    async fn filter_orders(
        &self,
        ws: &BinanceWs,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<usize>,
    ) -> Result<Vec<Order>> {
        let orders_map = ws.orders.read().await;

        let mut orders: Vec<Order> = if let Some(sym) = symbol {
            orders_map
                .get(sym)
                .map(|symbol_orders| symbol_orders.values().cloned().collect())
                .unwrap_or_default()
        } else {
            orders_map
                .values()
                .flat_map(|symbol_orders| symbol_orders.values().cloned())
                .collect()
        };

        if let Some(since_ts) = since {
            orders.retain(|order| order.timestamp.is_some_and(|ts| ts >= since_ts));
        }

        orders.sort_by(|a, b| {
            let ts_a = a.timestamp.unwrap_or(0);
            let ts_b = b.timestamp.unwrap_or(0);
            ts_b.cmp(&ts_a)
        });

        if let Some(lim) = limit {
            orders.truncate(lim);
        }

        Ok(orders)
    }

    /// Watches authenticated user trade updates
    pub async fn watch_my_trades(
        self: Arc<Self>,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<usize>,
        _params: Option<HashMap<String, Value>>,
    ) -> Result<Vec<Trade>> {
        const MAX_RETRIES: u32 = 100;

        let ws = self.create_authenticated_ws();
        ws.connect().await?;

        let mut retries = 0;

        while retries < MAX_RETRIES {
            if let Some(msg) = ws.client.receive().await {
                if msg.get("result").is_some() || msg.get("id").is_some() {
                    continue;
                }

                if let Some(event_type) = msg.get("e").and_then(|e| e.as_str()) {
                    if event_type == "executionReport" {
                        if let Ok(trade) = BinanceWs::parse_ws_trade(&msg) {
                            let symbol_key = trade.symbol.clone();

                            let mut trades_map = ws.my_trades.write().await;
                            let symbol_trades =
                                trades_map.entry(symbol_key).or_insert_with(VecDeque::new);

                            symbol_trades.push_front(trade);
                            if symbol_trades.len() > 1000 {
                                symbol_trades.pop_back();
                            }
                        }
                    }
                }
            } else {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }

            retries += 1;
        }

        ws.filter_my_trades(symbol, since, limit).await
    }

    /// Watches authenticated futures position updates
    pub async fn watch_positions(
        self: Arc<Self>,
        symbols: Option<Vec<String>>,
        since: Option<i64>,
        limit: Option<usize>,
        _params: Option<HashMap<String, Value>>,
    ) -> Result<Vec<Position>> {
        const MAX_RETRIES: u32 = 100;

        let ws = self.create_authenticated_ws();
        ws.connect().await?;

        let mut retries = 0;

        while retries < MAX_RETRIES {
            if let Some(msg) = ws.client.receive().await {
                if msg.get("result").is_some() || msg.get("id").is_some() {
                    continue;
                }

                if let Some(event_type) = msg.get("e").and_then(|e| e.as_str()) {
                    if event_type == "ACCOUNT_UPDATE" {
                        if let Some(account_data) = msg.get("a") {
                            if let Some(positions_array) =
                                account_data.get("P").and_then(|p| p.as_array())
                            {
                                for position_data in positions_array {
                                    if let Ok(position) =
                                        BinanceWs::parse_ws_position(position_data)
                                    {
                                        let symbol_key = position.symbol.clone();
                                        let side_key = position
                                            .side
                                            .clone()
                                            .unwrap_or_else(|| "both".to_string());

                                        let mut positions_map = ws.positions.write().await;
                                        let symbol_positions = positions_map
                                            .entry(symbol_key)
                                            .or_insert_with(HashMap::new);

                                        if position.contracts.unwrap_or(0.0).abs() < 0.000001 {
                                            symbol_positions.remove(&side_key);
                                            if symbol_positions.is_empty() {
                                                positions_map.remove(&position.symbol);
                                            }
                                        } else {
                                            symbol_positions.insert(side_key, position);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }

            retries += 1;
        }

        let symbols_ref = symbols.as_deref();
        ws.filter_positions(symbols_ref, since, limit).await
    }
}
