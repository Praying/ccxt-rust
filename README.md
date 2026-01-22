# <center> CCXT-Rust </center>
___

[![Rust](https://img.shields.io/badge/rust-1.91%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust CI](https://github.com/Praying/ccxt-rust/actions/workflows/rust.yml/badge.svg)](https://github.com/Praying/ccxt-rust/actions/workflows/rust.yml)
[![Documentation](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/ccxt-rust)

A professional-grade, type-safe Rust implementation of the CCXT library for cryptocurrency trading, providing unified
access to major exchanges.

[English](README.md) | [简体中文](README_CN.md)

## 🎯 Supported Exchanges

| Exchange        | Market Data | Trading | WebSocket |
|-----------------|-------------|---------|-----------|
| **Binance**     | ✅           | ✅       | ✅         |
| **Bitget**      | ✅           | ✅       | ✅         |
| **Hyperliquid** | ✅           | ✅       | ✅         |
| **OKX**         | ✅           | ✅       | ✅         |
| **Bybit**       | ✅           | ✅       | ✅         |

> **Legend**: ✅ Supported, 🚧 In Progress

## 🌟 Key Features

- **🛡️ Type-Safe & Async**: Built on `Tokio` and `rust_decimal` for safe, high-performance financial operations.
- **🔄 Unified Interface**: Consistent `Exchange` trait across all supported exchanges.
- **⚡ Real-Time**: Robust WebSocket support with automatic reconnection.
- **📦 Comprehensive Capability**:
  - **Market Data**: Tickers, Order Books, OHLCV, Trades.
  - **Trading**: Spot, Margin, Futures, Batched Orders, OCO.
  - **Account**: Balances, Transactions, Leverage management.

## 🚀 Quick Start

### Installation

```bash
cargo add ccxt-rust
```

### Basic Usage

```rust
use ccxt_exchanges::binance::Binance;
use ccxt_core::exchange::Exchange;
use rust_decimal_macros::dec;
use ccxt_core::types::{OrderType, OrderSide};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // 1. Initialize (using env vars recommended)
    let exchange = Binance::builder()
            .api_key(std::env::var("BINANCE_API_KEY").ok())
            .secret(std::env::var("BINANCE_SECRET").ok())
        .build()?;

  // 2. Fetch Market Data
    let ticker = exchange.fetch_ticker("BTC/USDT").await?;
    println!("BTC/USDT Price: {:?}", ticker.last);

  // 3. Place Order (if credentials provided)
  if exchange.has_private_api() {
    let order = exchange.create_order(
      "BTC/USDT",
      OrderType::Limit,
      OrderSide::Buy,
      dec!(0.001),
      Some(dec!(50000)),
    ).await?;
    println!("Order placed: {}", order.id);
    }

    Ok(())
}
```

For WebSocket examples and advanced usage (polymorphism), see the [`examples/`](examples/) directory.

## 🏗️ Architecture

The project is structured as a workspace:

- **`ccxt-core`**: Defines the unified `Exchange` and `WsExchange` traits, standard types, and error handling logic.
- **`ccxt-exchanges`**: Contains specific implementations (Binance, OKX, etc.).

## 🚩 Feature Flags

| Flag         | Description              | Default |
|--------------|--------------------------|---------|
| `rest`       | REST API support         | ✅       |
| `websocket`  | WebSocket support        | ✅       |
| `rustls-tls` | Use RustLS (recommended) | ✅       |
| `native-tls` | Use OpenSSL/Native TLS   | ❌       |

## 🛠️ Development

```bash
# Run tests
cargo test

# Check code quality
cargo clippy --all-targets -- -D warnings

# Build documentation
cargo doc --open
```

## 📝 License & Support

MIT License. See [LICENSE](LICENSE).

- **Issues**: [GitHub Issues](https://github.com/Praying/ccxt-rust/issues)
- **Docs**: [docs.rs](https://docs.rs/ccxt-rust)

## ⚠️ Disclaimer

This project is for educational and research purposes only. The authors and contributors are not responsible for any
financial losses or damages arising from the use of this software. Cryptocurrency trading involves high risk; please
trade responsibly.

---
**Status**: 🚧 Active Development (v0.1.3) | **Donations (BSC)**: `0x8e5d858f92938b028065d39450421d0e080d15f7`
