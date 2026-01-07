# <center> CCXT-Rust </center>
___

[![Rust](https://img.shields.io/badge/rust-1.91%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust CI](https://github.com/Praying/ccxt-rust/actions/workflows/rust.yml/badge.svg)](https://github.com/Praying/ccxt-rust/actions/workflows/rust.yml)
[![Security Audit](https://github.com/Praying/ccxt-rust/actions/workflows/rust.yml/badge.svg)](https://github.com/Praying/ccxt-rust/actions/workflows/rust.yml)
[![Documentation](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/ccxt-rust)

A professional-grade Rust implementation of the CCXT (CryptoCurrency eXchange Trading) library, providing unified, type-safe access to Major cryptocurrency exchanges with high-performance async operations.

[English](README.md) | [简体中文](README_CN.md)

## 🎯 Supported Exchanges

| Exchange | Public API (Market Data) | Private API (Trading) | WebSocket |
|----------|--------------------------|-----------------------|-----------|
| **Binance** | ✅ | ✅ | ✅ |
| **Bitget** | ✅ | ✅ | ✅ |
| **Hyperliquid** | ✅ | ✅ | ✅ |
| **OKX** | ✅ | ✅ | ✅ |
| **Bybit** | ✅ | ✅ | ✅ |

> **Legend**: ✅ Supported, 🚧 In Progress, 🔄 Planned

## 🌟 Features

### Core Capabilities
- **✅ Type-Safe Trading Operations** - Leverage Rust's strong type system for compile-time safety
- **✅ Async/Await Architecture** - Built on Tokio for efficient, non-blocking I/O operations
- **✅ Precise Financial Calculations** - Using `rust_decimal` for accurate monetary computations
- **✅ Comprehensive Error Handling** - Structured error types with full context propagation
- **✅ REST API Support** - Complete REST API implementation for exchange operations
- **✅ WebSocket Real-Time Data** - Live market data streaming with automatic reconnection
- **✅ Multi-Exchange Support** - Unified interface across multiple cryptocurrency exchanges

### Advanced Features
- **Market Data Operations**
  - Fetch tickers, order books, and OHLCV data
  - Real-time market data streaming via WebSocket
  - Advanced market data with depth and aggregation

- **Order Management**
  - Create, cancel, and modify orders
  - Support for market, limit, and conditional orders
  - OCO (One-Cancels-Other) order support
  - Batch order operations

- **Account Management**
  - Balance inquiries and account information
  - Deposit and withdrawal operations
  - Transaction history and ledger access
  - Fee management and calculation

- **Trading Features**
  - Spot trading
  - Margin trading (cross and isolated)
  - Futures trading with position management
  - Leverage and margin management

- **WebSocket Features**
  - Real-time order book updates
  - Live trade streaming
  - Account balance updates
  - Order status updates
  - Position updates for futures

## 🏗️ Architecture

The project follows a clean, modular workspace architecture with a unified Exchange trait:

### Project Structure

```
ccxt-rust/
├── ccxt-core/              # Core types, traits, and error handling
│   ├── types/              # Market, Order, Trade, Ticker, etc.
│   ├── exchange.rs         # Unified Exchange trait
│   ├── ws_exchange.rs      # WebSocket Exchange trait
│   ├── error/              # Comprehensive error types
│   ├── base_exchange/      # Base exchange functionality
│   ├── http_client/        # HTTP client with retry and circuit breaker
│   ├── ws_client/          # WebSocket client with auto-reconnect
│   ├── auth/               # Authentication and signing
│   └── ...
├── ccxt-exchanges/         # Exchange-specific implementations
│   ├── binance/            # Binance exchange implementation
│   ├── okx/                # OKX exchange implementation
│   ├── bybit/              # Bybit exchange implementation
│   ├── bitget/             # Bitget exchange implementation
│   └── hyperliquid/        # Hyperliquid exchange implementation
├── examples/               # Comprehensive usage examples
├── tests/                  # Integration tests
└── docs/                   # Detailed documentation
```

### Module Relationships

```
┌─────────────────────────────────────────────────────────────┐
│                     Application Code                        │
│  (uses exchanges through unified Exchange trait interface)  │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                  Unified Exchange Trait                     │
│  (ccxt-core::exchange::Exchange)                            │
│  - Provides polymorphic interface for all exchanges        │
│  - Capability-based feature discovery                       │
│  - Market data, trading, account management methods        │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                    Base Exchange Layer                      │
│  (ccxt-core::base_exchange::BaseExchange)                   │
│  - Common functionality shared across exchanges             │
│  - Market caching, precision handling, symbol parsing       │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│              Exchange Implementations                        │
│  (ccxt-exchanges::binance, okx, bybit, ...)                 │
│  - Exchange-specific API clients                            │
│  - REST and WebSocket implementations                       │
│  - Custom request parsing and authentication                │
└─────────────────────────────────────────────────────────────┘
```

### Key Design Principles

1. **Trait-Based Abstraction**: Unified `Exchange` trait allows polymorphic usage
2. **Capability Discovery**: Runtime feature detection via `ExchangeCapabilities`
3. **Type Safety**: Strong typing with `rust_decimal` for financial calculations
4. **Error Handling**: Comprehensive error types with context preservation
5. **Async First**: Built on Tokio for efficient async operations

### Unified Exchange Trait

The `Exchange` trait in `ccxt-core` provides a unified interface for all exchanges:

```rust
use ccxt_core::exchange::{Exchange, ExchangeCapabilities, BoxedExchange};

// Use any exchange through the unified interface
async fn fetch_price(exchange: &dyn Exchange, symbol: &str) -> Result<Decimal, Error> {
    // Check capability before calling
    if !exchange.capabilities().fetch_ticker() {
        return Err(Error::not_implemented("fetch_ticker"));
    }
    
    let ticker = exchange.fetch_ticker(symbol).await?;
    ticker.last.ok_or_else(|| Error::invalid_response("No last price"))
}

// Use multiple exchanges polymorphically
async fn compare_prices(exchanges: &[BoxedExchange], symbol: &str) {
    for exchange in exchanges {
        println!("{}: {:?}", exchange.name(), fetch_price(exchange.as_ref(), symbol).await);
    }
}
```

### WebSocket Streaming

The `WsExchange` trait provides real-time data streaming:

```rust
use ccxt_core::ws_exchange::{WsExchange, FullExchange};
use futures::StreamExt;

async fn watch_market(exchange: &dyn WsExchange, symbol: &str) {
    exchange.ws_connect().await.unwrap();
    
    let mut stream = exchange.watch_ticker(symbol).await.unwrap();
    while let Some(Ok(ticker)) = stream.next().await {
        println!("Price: {:?}", ticker.last);
    }
}
```

## 🚀 Quick Start

**New to ccxt-rust?** Start with our [5-minute Quick Start Guide](QUICKSTART.md) 📖

### Prerequisites

- Rust 1.91+ or later
- Cargo (latest stable)

### Installation

Add via command line:

```bash
cargo add ccxt-rust
```

Or add to your `Cargo.toml`:

```toml
[dependencies]
ccxt-core = { path = "ccxt-core" }
ccxt-exchanges = { path = "ccxt-exchanges" }
tokio = { version = "1.35", features = ["full"] }
rust_decimal = "1.39"
futures = "0.3"
```

### Basic Usage with Builder Pattern

```rust
use ccxt_exchanges::binance::Binance;
use ccxt_core::exchange::Exchange;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize exchange using the builder pattern (recommended)
    let exchange = Binance::builder()
        .api_key("your_api_key")
        .secret("your_secret")
        .sandbox(false)  // Use production API
        .build()?;

    // Fetch ticker through the unified Exchange trait
    let ticker = exchange.fetch_ticker("BTC/USDT").await?;
    println!("BTC/USDT Price: {:?}", ticker.last);

    // Fetch order book
    let orderbook = exchange.fetch_order_book("BTC/USDT", Some(10)).await?;
    if let Some(best_bid) = orderbook.bids.first() {
        println!("Best bid: {}", best_bid.price);
    }
    if let Some(best_ask) = orderbook.asks.first() {
        println!("Best ask: {}", best_ask.price);
    }

    // Place an order (requires API credentials)
    use ccxt_core::types::{OrderType, OrderSide};
    use rust_decimal_macros::dec;

    let order = exchange.create_order(
        "BTC/USDT",
        OrderType::Limit,
        OrderSide::Buy,
        dec!(0.001),
        Some(dec!(40000)),
    ).await?;
    println!("Order placed: {}", order.id);

    Ok(())
}
```

### Using Exchanges Polymorphically

```rust
use ccxt_core::exchange::{Exchange, BoxedExchange};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create exchange as a trait object
    let exchange: BoxedExchange = Box::new(
        ccxt_exchanges::binance::Binance::builder().build()?
    );

    // Use through the unified interface
    println!("Exchange: {} ({})", exchange.name(), exchange.id());
    println!("Capabilities: {:?}", exchange.capabilities());

    // Check capabilities before calling methods
    if exchange.capabilities().fetch_ticker() {
        let ticker = exchange.fetch_ticker("BTC/USDT").await?;
        println!("Price: {:?}", ticker.last);
    }

    Ok(())
}
```

### WebSocket Streaming

```rust
use ccxt_exchanges::binance::Binance;
use ccxt_core::ws_exchange::WsExchange;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize exchange
    let exchange = Binance::builder().build()?;

    // Watch real-time ticker updates using the WsExchange trait
    let mut stream = exchange.watch_ticker("BTC/USDT").await?;

    while let Some(result) = stream.next().await {
        match result {
            Ok(ticker) => println!("Price: {:?}", ticker.last),
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    Ok(())
}
```

**More examples**: See the [QUICKSTART.md](QUICKSTART.md) guide for detailed tutorials!

## 📚 Examples

The project includes comprehensive examples covering all major features:

- **`basic_usage.rs`** - Getting started with the library
- **`binance_market_data_example.rs`** - Market data operations
- **`binance_order_management_example.rs`** - Order creation and management
- **`binance_account_example.rs`** - Account operations
- **`binance_margin_example.rs`** - Margin trading
- **`binance_futures_example.rs`** - Futures trading
- **`binance_ws_example.rs`** - WebSocket streaming
- **`binance_conditional_orders_example.rs`** - Conditional orders
- **`binance_deposit_withdrawal_example.rs`** - Deposit/withdrawal operations

Run any example:

```bash
cargo run --example basic_usage
cargo run --example binance_ws_example
```



## 🚩 Feature Flags

Optimize your build by selecting only the features you need in `Cargo.toml`:

- **`default`**: Enables `rest`, `websocket`, and `rustls-tls`.
- **`rest`**: Enables REST API support.
- **`websocket`**: Enables WebSocket support.
- **`rustls-tls`**: Uses `rustls` for TLS (default, recommended).
- **`native-tls`**: Uses platform-native TLS (OpenSSL/Schannel/Secure Transport).
- **`compression`**: Enables GZIP compression for HTTP requests.
- **`full`**: Enables all features.

## 🔧 Configuration

### Environment Variables

Create a `.env` file from the template:

```bash
cp .env.example .env
```

Key configuration options:

```bash
# API Credentials
BINANCE_API_KEY=your_api_key_here
BINANCE_API_SECRET=your_secret_here

# Testing
ENABLE_PRIVATE_TESTS=false
ENABLE_INTEGRATION_TESTS=false
USE_MOCK_DATA=true
TEST_SYMBOL=BTC/USDT

# Logging
RUST_LOG=info
```

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test suite
cargo test -p ccxt-core
cargo test -p ccxt-exchanges

# Run integration tests
cargo test --test binance_integration_test

# Run with live API (requires credentials)
ENABLE_INTEGRATION_TESTS=true cargo test
```

## 📖 Documentation

- **[API Documentation](docs/)** - Detailed API reference
- **[Testing Guide](docs/TESTING.md)** - Comprehensive testing documentation
- **[Implementation Plans](docs/)** - Feature implementation roadmaps
- **[Comparison Analysis](docs/GO_RUST_COMPARISON_ANALYSIS.md)** - Go vs Rust implementation

Generate local documentation:

```bash
cargo doc --open
```


## 🛠️ Development

### Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Build specific package
cargo build -p ccxt-core
cargo build -p ccxt-exchanges
```

### Code Quality

```bash
# Format code
cargo fmt

# Run linter
cargo clippy --all-targets --all-features

# Strict linting (no warnings)
cargo clippy --all-targets --all-features -- -D warnings

# Check compilation
cargo check --all-features
```

## 🔐 Security

- **Never commit API keys or secrets** - Always use environment variables
- **Secure credential storage** - Use system keychains or encrypted vaults
- **Rate limiting** - Built-in rate limiting to prevent API bans
- **Input validation** - All inputs are validated before API calls
- **HTTPS only** - All communications use TLS encryption

## 🤝 Contributing

Contributions are welcome! Please follow these guidelines:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Follow Rust best practices and project conventions
4. Add tests for new features
5. Ensure all tests pass (`cargo test`)
6. Run formatting and linting (`cargo fmt && cargo clippy`)
7. Commit your changes (`git commit -m 'Add amazing feature'`)
8. Push to the branch (`git push origin feature/amazing-feature`)
9. Open a Pull Request

### Development Conventions

- **Code Style**: Rust 2024 edition, 100-character line width
- **Testing**: Minimum 80% test coverage
- **Documentation**: All public APIs must have documentation
- **Error Handling**: Use `thiserror` for custom errors, `anyhow` for context

## 📊 Performance

Built for high performance:
- **Async I/O**: Non-blocking operations using Tokio
- **Zero-copy parsing**: Efficient JSON deserialization
- **Connection pooling**: Reused HTTP connections
- **Optimized builds**: LTO and single codegen unit for releases
- **Benchmarks**: Criterion-based performance benchmarks

## 🐛 Troubleshooting

### Common Issues

1. **Compilation errors**
   - Ensure Rust 1.91+ is installed: `rustc --version`
   - Update dependencies: `cargo update`
   - Clean build: `cargo clean && cargo build`

2. **API authentication failures**
   - Verify API keys in `.env` file
   - Check API key permissions on exchange
   - Ensure system clock is synchronized

3. **Rate limiting**
   - Reduce request frequency
   - Use WebSocket for real-time data
   - Check exchange-specific rate limits

For more help, see [documentation](docs/) or open an issue.

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Inspired by the original [CCXT](https://github.com/ccxt/ccxt) library
- Built with amazing Rust ecosystem libraries
- Community contributors and testers

## 📞 Contact & Support

- **Issues**: [GitHub Issues](https://github.com/Praying/ccxt-rust/issues)
- **Discussions**: [GitHub Discussions](https://github.com/Praying/ccxt-rust/discussions)
- **Documentation**: [Project Docs](docs/)

---

**Status**: 🚧 Active Development | **Version**: 0.1.2 | **Updated**: 2026-01

⚠️ **Note**: This library is under active development. APIs may change before v1.0. Not recommended for production use yet.