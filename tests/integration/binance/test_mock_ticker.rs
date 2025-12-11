use ccxt_core::ExchangeConfig;
use ccxt_exchanges::binance::Binance;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_fetch_ticker_mock() {
    // Start a mock server
    let mock_server = MockServer::start().await;

    // Configure Binance with the mock server URL
    let config = ExchangeConfig::builder()
        .url_override("public", mock_server.uri())
        .build();

    let binance = Binance::new(config).expect("Failed to create Binance client");

    // Mock the exchangeInfo endpoint (required for load_markets)
    Mock::given(method("GET"))
        .and(path("/exchangeInfo"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(
                serde_json::from_str::<serde_json::Value>(
                    r#"{
                    "timezone": "UTC",
                    "serverTime": 1592882214236,
                    "rateLimits": [],
                    "exchangeFilters": [],
                    "symbols": [
                        {
                            "symbol": "BTCUSDT",
                            "status": "TRADING",
                            "baseAsset": "BTC",
                            "baseAssetPrecision": 8,
                            "quoteAsset": "USDT",
                            "quotePrecision": 8,
                            "quoteAssetPrecision": 8,
                            "baseCommissionPrecision": 8,
                            "quoteCommissionPrecision": 8,
                            "orderTypes": [
                                "LIMIT",
                                "LIMIT_MAKER",
                                "MARKET",
                                "STOP_LOSS_LIMIT",
                                "TAKE_PROFIT_LIMIT"
                            ],
                            "icebergAllowed": true,
                            "ocoAllowed": true,
                            "quoteOrderQtyMarketAllowed": true,
                            "isSpotTradingAllowed": true,
                            "isMarginTradingAllowed": true,
                            "filters": [],
                            "permissions": [
                                "SPOT",
                                "MARGIN"
                            ]
                        }
                    ]
                }"#,
                )
                .expect("Failed to parse exchange info JSON"),
            ),
        )
        .mount(&mock_server)
        .await;

    // Mock the ticker endpoint
    Mock::given(method("GET"))
        .and(path("/ticker/24hr"))
        .and(query_param("symbol", "BTCUSDT"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(
                serde_json::from_str::<serde_json::Value>(
                    r#"{
                    "symbol": "BTCUSDT",
                    "priceChange": "-94.99999800",
                    "priceChangePercent": "-95.960",
                    "weightedAvgPrice": "0.29628482",
                    "prevClosePrice": "0.10002000",
                    "lastPrice": "4.00000200",
                    "lastQty": "200.00000000",
                    "bidPrice": "4.00000000",
                    "bidQty": "100.00000000",
                    "askPrice": "4.00000200",
                    "askQty": "100.00000000",
                    "openPrice": "99.00000000",
                    "highPrice": "100.00000000",
                    "lowPrice": "0.10000000",
                    "volume": "8913.30000000",
                    "quoteVolume": "15.30000000",
                    "openTime": 1499783499040,
                    "closeTime": 1499869899040,
                    "firstId": 28385,
                    "lastId": 28460,
                    "count": 76
                }"#,
                )
                .expect("Failed to parse ticker JSON"),
            ),
        )
        .mount(&mock_server)
        .await;

    // Load markets first
    binance
        .load_markets(true)
        .await
        .expect("Failed to load markets");

    // Call fetch_ticker
    let ticker = binance
        .fetch_ticker("BTC/USDT", ccxt_core::types::TickerParams::default())
        .await
        .expect("Failed to fetch ticker");

    // Verify the result
    assert_eq!(ticker.symbol, "BTC/USDT");
    assert_eq!(
        ticker.last.expect("Last price should exist").to_string(),
        "4.00000200"
    );
    assert_eq!(
        ticker.high.expect("High price should exist").to_string(),
        "100.00000000"
    );
    assert_eq!(
        ticker.low.expect("Low price should exist").to_string(),
        "0.10000000"
    );
    assert_eq!(
        ticker
            .base_volume
            .expect("Base volume should exist")
            .to_string(),
        "8913.30000000"
    );
}
