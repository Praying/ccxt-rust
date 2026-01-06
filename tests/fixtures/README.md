# Test Fixtures

This directory contains test fixtures for integration tests.

## Structure

```
fixtures/
├── README.md
├── binance/
│   ├── tickers.json
│   ├── orderbooks.json
│   ├── trades.json
│   ├── markets.json
│   ├── balances.json
│   └── orders.json
├── bybit/
│   ├── tickers.json
│   ├── orderbooks.json
│   ├── trades.json
│   └── markets.json
├── okx/
│   ├── tickers.json
│   ├── orderbooks.json
│   ├── trades.json
│   └── markets.json
└── hyperliquid/
    ├── tickers.json
    ├── orderbooks.json
    ├── trades.json
    └── markets.json
```

## Usage

Fixtures can be loaded using the `ccxt_core::test_config::TestConfig` system:

```rust
use ccxt_core::test_config::TestConfig;

fn load_fixture() {
    let config = TestConfig::from_env();
    
    // Load ticker fixture
    let ticker = config.load_fixture::<Ticker>("binance/tickers.json");
    
    // Load orderbook fixture
    let orderbook = config.load_fixture::<OrderBook>("binance/orderbooks.json");
    
    // Load trades fixture
    let trades = config.load_fixture::<Vec<Trade>>("binance/trades.json");
}
```

## Fixture Format

### Ticker Fixture

```json
{
  "symbol": "BTC/USDT",
  "last": 50000.00,
  "bid": 49999.00,
  "ask": 50001.00,
  "high": 51000.00,
  "low": 49000.00,
  "volume": 1000.5,
  "timestamp": 1234567890000
}
```

### Orderbook Fixture

```json
{
  "symbol": "BTC/USDT",
  "bids": [
    ["49999.00", "1.0"],
    ["49998.00", "2.0"]
  ],
  "asks": [
    ["50001.00", "1.0"],
    ["50002.00", "2.0"]
  ],
  "timestamp": 1234567890000
}
```

### Trade Fixture

```json
{
  "id": "12345",
  "symbol": "BTC/USDT",
  "price": 50000.00,
  "amount": 0.1,
  "side": "buy",
  "timestamp": 1234567890000
}
```

### Market Fixture

```json
{
  "symbol": "BTC/USDT",
  "base": "BTC",
  "quote": "USDT",
  "active": true,
  "type": "spot",
  "spot": true,
  "margin": false,
  "future": false,
  "contract": false,
  "precision": {
    "price": 2,
    "amount": 8
  },
  "limits": {
    "price": {
      "min": 0.01,
      "max": 1000000.00
    },
    "amount": {
      "min": 0.00001,
      "max": 9000.00
    }
  }
}
```

## Updating Fixtures

To update fixtures with real data:

1. Run the integration test with live API access
2. Capture the API response
3. Save the response to the appropriate fixture file
4. Verify the fixture format matches the expected structure

## Notes

- Fixtures should be representative of real API responses
- Fixtures should be kept up-to-date with API changes
- Use realistic data values for better test coverage
- Include edge cases and error scenarios where appropriate
