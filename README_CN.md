# <center> CCXT-Rust </center>

___

[![Rust](https://img.shields.io/badge/rust-1.91%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust CI](https://github.com/Praying/ccxt-rust/actions/workflows/rust.yml/badge.svg)](https://github.com/Praying/ccxt-rust/actions/workflows/rust.yml)
[![Documentation](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/ccxt-rust)

CCXT 库的专业级 Rust 实现，提供统一、类型安全的接口访问主流加密货币交易所。

[English](README.md) | [简体中文](README_CN.md)

## 🎯 支持的交易所

| 交易所             | 市场数据 | 交易 API | WebSocket |
|-----------------|------|--------|-----------|
| **Binance**     | ✅    | ✅      | ✅         |
| **Bitget**      | ✅    | ✅      | ✅         |
| **Hyperliquid** | ✅    | ✅      | ✅         |
| **OKX**         | ✅    | ✅      | ✅         |
| **Bybit**       | ✅    | ✅      | ✅         |

> **图例**: ✅ 已支持, 🚧 开发中

## 🌟 核心特性

- **🛡️ 类型安全与异步**: 基于 `Tokio` 和 `rust_decimal` 构建，确保高性能与金融计算安全。
- **🔄 统一接口**: 所有交易所均实现统一的 `Exchange` trait。
- **⚡ 实时数据**: 强大的 WebSocket 支持，具备自动重连功能。
- **📦 功能全面**:
  - **行情**: Ticker, 深度图, K线 (OHLCV), 成交记录。
  - **交易**: 现货, 杠杆, 合约, 批量下单, OCO。
  - **账户**: 余额查询, 资金划转, 杠杆管理。

## 🚀 快速开始

### 安装

```bash
cargo add ccxt-rust
```

### 基本用法

```rust
use ccxt_exchanges::binance::Binance;
use ccxt_core::exchange::Exchange;
use rust_decimal_macros::dec;
use ccxt_core::types::{OrderType, OrderSide};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // 1. 初始化 (建议使用环境变量)
    let exchange = Binance::builder()
            .api_key(std::env::var("BINANCE_API_KEY").ok())
            .secret(std::env::var("BINANCE_SECRET").ok())
        .build()?;

  // 2. 获取行情
    let ticker = exchange.fetch_ticker("BTC/USDT").await?;
    println!("BTC/USDT 价格: {:?}", ticker.last);

  // 3. 下单 (如提供了 API Key)
  if exchange.has_private_api() {
    let order = exchange.create_order(
      "BTC/USDT",
      OrderType::Limit,
      OrderSide::Buy,
      dec!(0.001),
      Some(dec!(50000)),
    ).await?;
    println!("下单成功: {}", order.id);
  }

  Ok(())
}
```

更多 WebSocket 和高级用法示例请查看 [`examples/`](examples/) 目录。

## 🏗️ 架构

项目采用模块化工作空间结构：

- **`ccxt-core`**: 定义统一的 `Exchange` 和 `WsExchange` trait、标准类型及错误处理逻辑。
- **`ccxt-exchanges`**: 包含具体交易所的实现 (Binance, OKX 等)。

## 🚩 功能标志 (Feature Flags)

| 标志           | 说明                  | 默认开启 |
|--------------|---------------------|------|
| `rest`       | REST API 支持         | ✅    |
| `websocket`  | WebSocket 支持        | ✅    |
| `rustls-tls` | 使用 RustLS (推荐)      | ✅    |
| `native-tls` | 使用 OpenSSL/系统原生 TLS | ❌    |

## 🛠️ 开发与测试

```bash
# 运行测试
cargo test

# 代码检查
cargo clippy --all-targets -- -D warnings

# 生成文档
cargo doc --open
```

## 📝 许可证与支持

MIT License. 详见 [LICENSE](LICENSE).

- **问题反馈**: [GitHub Issues](https://github.com/Praying/ccxt-rust/issues)
- **文档**: [docs.rs](https://docs.rs/ccxt-rust)

## ⚠️ 免责声明

本项目仅供学习和研究使用。作者和贡献者不对因使用本软件而产生的任何财务损失或损害负责。加密货币交易风险极高，请谨慎交易。

---
**状态**: 🚧 开发中 (v0.1.4) | **捐赠 (BSC)**: `0x8e5d858f92938b028065d39450421d0e080d15f7`
