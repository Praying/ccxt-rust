# <center> CCXT-Rust </center>

[![Rust](https://img.shields.io/badge/rust-1.91%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust CI](https://github.com/Praying/ccxt-rust/actions/workflows/rust.yml/badge.svg)](https://github.com/Praying/ccxt-rust/actions/workflows/rust.yml)
[![Security Audit](https://github.com/Praying/ccxt-rust/actions/workflows/rust.yml/badge.svg)](https://github.com/Praying/ccxt-rust/actions/workflows/rust.yml)
[![Documentation](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/ccxt-rust)

CCXT (CryptoCurrency eXchange Trading) 库的专业级 Rust 实现，提供统一、类型安全的接口访问主流的加密货币交易所，具有高性能异步操作特性。

[English](README.md) | 简体中文

## 🎯 支持的交易所

| 交易所 | 公共 API (行情数据) | 私有 API (交易) | WebSocket |
|----------|--------------------------|-----------------------|-----------|
| **Binance** | ✅ | ✅ | ✅ |
| **Bitget** | ✅ | ✅ | ✅ |
| **Hyperliquid** | ✅ | ✅ | ✅ |
| **OKX** | ✅ | ✅ | ✅ |
| **Bybit** | ✅ | ✅ | ✅ |

> **图例**: ✅ 已支持, 🚧 开发中, 🔄 计划中


## 🌟 特性

### 核心能力
- **✅ 类型安全的交易操作** - 利用 Rust 强大的类型系统实现编译时安全
- **✅ 异步/等待架构** - 基于 Tokio 构建，实现高效的非阻塞 I/O 操作
- **✅ 精确的金融计算** - 使用 `rust_decimal` 进行精确的货币计算
- **✅ 全面的错误处理** - 结构化错误类型，完整的上下文传播
- **✅ REST API 支持** - 完整的 REST API 实现，支持各种交易所操作
- **✅ WebSocket 实时数据** - 实时市场数据流，支持自动重连
- **✅ 多交易所支持** - 跨多个加密货币交易所的统一接口

### 高级功能
- **市场数据操作**
  - 获取行情、订单簿和 OHLCV 数据
  - 通过 WebSocket 实时市场数据流
  - 高级市场数据，包含深度和聚合功能

- **订单管理**
  - 创建、取消和修改订单
  - 支持市价、限价和条件订单
  - OCO（一单取消另一单）订单支持
  - 批量订单操作

- **账户管理**
  - 余额查询和账户信息
  - 充值和提现操作
  - 交易历史和账本访问
  - 手续费管理和计算

- **交易功能**
  - 现货交易
  - 保证金交易（全仓和逐仓）
  - 期货交易及仓位管理
  - 杠杆和保证金管理

- **WebSocket 功能**
  - 实时订单簿更新
  - 实时交易流
  - 账户余额更新
  - 订单状态更新
  - 期货仓位更新

## 🏗️ 架构

项目采用清晰的模块化工作空间架构，并提供统一的 Exchange trait：

```
ccxt-rust/
├── ccxt-core/              # 核心类型、trait 和错误处理
│   ├── types/              # Market、Order、Trade、Ticker 等
│   ├── exchange.rs         # 统一 Exchange trait
│   ├── ws_exchange.rs      # WebSocket Exchange trait
│   ├── error.rs            # 全面的错误类型
│   └── base_exchange.rs    # 基础交易所功能
├── ccxt-exchanges/         # 交易所特定实现
│   └── binance/            # Binance 交易所实现
│       ├── mod.rs          # Binance 主结构体
│       ├── builder.rs      # BinanceBuilder
│       ├── exchange_impl.rs # Exchange trait 实现
│       ├── ws_exchange_impl.rs # WsExchange trait 实现
│       ├── rest/           # REST API 客户端模块
│       ├── ws.rs           # WebSocket 客户端
│       ├── parser.rs       # 响应解析
│       └── auth.rs         # 认证
├── examples/               # 全面的使用示例
├── tests/                  # 集成测试
└── docs/                   # 详细文档
```

### 统一 Exchange Trait

`ccxt-core` 中的 `Exchange` trait 为所有交易所提供了统一的接口：

```rust
use ccxt_core::exchange::{Exchange, ExchangeCapabilities, BoxedExchange};

// 通过统一接口使用任何交易所
async fn fetch_price(exchange: &dyn Exchange, symbol: &str) -> Result<Decimal, Error> {
    // 调用前检查功能支持情况
    if !exchange.capabilities().fetch_ticker() {
        return Err(Error::not_implemented("fetch_ticker"));
    }
    
    let ticker = exchange.fetch_ticker(symbol).await?;
    ticker.last.ok_or_else(|| Error::invalid_response("No last price"))
}

// 多态地使用多个交易所
async fn compare_prices(exchanges: &[BoxedExchange], symbol: &str) {
    for exchange in exchanges {
        println!("{}: {:?}", exchange.name(), fetch_price(exchange.as_ref(), symbol).await);
    }
}
```

### WebSocket 流式传输

`WsExchange` trait 提供实时数据流功能：

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

## 🚀 快速开始

### 前置要求

- Rust 1.91+ 或更高版本
- Cargo（最新稳定版）

### 安装

通过命令行添加：

```bash
cargo add ccxt-rust
```

或者在你的 `Cargo.toml` 中添加：

```toml
[dependencies]
ccxt-core = { path = "ccxt-core" }
ccxt-exchanges = { path = "ccxt-exchanges" }
tokio = { version = "1.35", features = ["full"] }
rust_decimal = "1.39"
futures = "0.3"
```

### 集成示例：类型安全的交易机器人

此示例展示了如何将 `ccxt-rust` 集成到你自己的结构体中，利用 Rust 的类型系统实现编译时安全。

```rust
use ccxt_core::exchange::Exchange;
use ccxt_exchanges::binance::Binance;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::sync::Arc;

// 定义一个可以与任何交易所实现一起工作的交易机器人
struct TradingBot<E: Exchange> {
    exchange: E,
    symbol: String,
    target_price: Decimal,
}

impl<E: Exchange> TradingBot<E> {
    pub fn new(exchange: E, symbol: &str, target_price: Decimal) -> Self {
        Self {
            exchange,
            symbol: symbol.to_string(),
            target_price,
        }
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Checking {} on {}...", self.symbol, self.exchange.name());

        // 编译时检查：编译器确保 'fetch_ticker' 返回
        // 强类型的 'Ticker' 结构体，防止类型错误。
        let ticker = self.exchange.fetch_ticker(&self.symbol).await?;
        
        if let Some(last_price) = ticker.last {
            println!("Current price: {}", last_price);
            
            // 类型安全比较：rust_decimal 确保精度
            if last_price <= self.target_price {
                println!("Target price reached! Executing buy strategy...");
                // execute_buy_order()...
            }
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 初始化交易所
    let binance = Binance::builder().build()?;

    // 2. 创建机器人（编译器验证 'binance' 实现了 'Exchange'）
    let bot = TradingBot::new(binance, "BTC/USDT", dec!(50000));

    // 3. 运行策略
    bot.run().await?;

    Ok(())
}
```

### 多态地使用交易所

```rust
use ccxt_core::exchange::{Exchange, BoxedExchange};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建交易所作为 trait 对象
    let exchange: BoxedExchange = Box::new(
        ccxt_exchanges::binance::Binance::builder().build()?
    );
    
    // 通过统一接口使用
    println!("Exchange: {} ({})", exchange.name(), exchange.id());
    println!("Capabilities: {:?}", exchange.capabilities());
    
    // 调用方法前检查功能支持
    if exchange.capabilities().fetch_ticker() {
        let ticker = exchange.fetch_ticker("BTC/USDT").await?;
        println!("Price: {:?}", ticker.last);
    }
    
    Ok(())
}
```

### WebSocket 流式传输

```rust
use ccxt_exchanges::binance::Binance;
use ccxt_core::ws_exchange::WsExchange;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化交易所
    let exchange = Binance::builder().build()?;

    // 使用 WsExchange trait 监听实时行情更新
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

## 📚 示例

项目包含涵盖所有主要功能的全面示例：

- **`basic_usage.rs`** - 库入门使用
- **`binance_market_data_example.rs`** - 市场数据操作
- **`binance_order_management_example.rs`** - 订单创建和管理
- **`binance_account_example.rs`** - 账户操作
- **`binance_margin_example.rs`** - 保证金交易
- **`binance_futures_example.rs`** - 期货交易
- **`binance_ws_example.rs`** - WebSocket 流式传输
- **`binance_conditional_orders_example.rs`** - 条件订单
- **`binance_deposit_withdrawal_example.rs`** - 充值/提现操作

运行任何示例：

```bash
cargo run --example basic_usage
cargo run --example binance_ws_example
```


## 🚩 功能标志 (Feature Flags)

通过在 `Cargo.toml` 中选择所需的功能来优化构建：

- **`default`**: 启用 `rest`, `websocket`, 和 `rustls-tls`。
- **`rest`**: 启用 REST API 支持。
- **`websocket`**: 启用 WebSocket 支持。
- **`rustls-tls`**: 使用 `rustls` 进行 TLS（默认，推荐）。
- **`native-tls`**: 使用平台原生 TLS (OpenSSL/Schannel/Secure Transport)。
- **`compression`**: 启用 HTTP 请求的 GZIP 压缩。
- **`full`**: 启用所有功能。

## 🔧 配置

### 环境变量

从模板创建 `.env` 文件：

```bash
cp .env.example .env
```

主要配置选项：

```bash
# API 凭证
BINANCE_API_KEY=your_api_key_here
BINANCE_API_SECRET=your_secret_here

# 测试
ENABLE_PRIVATE_TESTS=false
ENABLE_INTEGRATION_TESTS=false
USE_MOCK_DATA=true
TEST_SYMBOL=BTC/USDT

# 日志
RUST_LOG=info
```

## 🧪 测试

```bash
# 运行所有测试
cargo test

# 带输出运行测试
cargo test -- --nocapture

# 运行特定测试套件
cargo test -p ccxt-core
cargo test -p ccxt-exchanges

# 运行集成测试
cargo test --test binance_integration_test

# 使用真实 API 运行（需要凭证）
ENABLE_INTEGRATION_TESTS=true cargo test
```

## 📖 文档

- **[API 文档](docs/)** - 详细的 API 参考
- **[测试指南](docs/TESTING.md)** - 全面的测试文档
- **[实现计划](docs/)** - 功能实现路线图
- **[对比分析](docs/GO_RUST_COMPARISON_ANALYSIS.md)** - Go vs Rust 实现对比

生成本地文档：

```bash
cargo doc --open
```

## 🛠️ 开发

### 构建

```bash
# Debug 构建
cargo build

# Release 构建（优化）
cargo build --release

# 构建特定包
cargo build -p ccxt-core
cargo build -p ccxt-exchanges
```

### 代码质量

```bash
# 格式化代码
cargo fmt

# 运行 linter
cargo clippy --all-targets --all-features

# 严格 linting (无警告)
cargo clippy --all-targets --all-features -- -D warnings

# 检查编译
cargo check --all-features
```

## 🔐 安全

- **切勿提交 API 密钥或机密信息** - 始终使用环境变量
- **安全的凭证存储** - 使用系统密钥链或加密保管库
- **速率限制** - 内置速率限制以防止 API 封禁
- **输入验证** - 所有输入在 API 调用前都经过验证
- **仅限 HTTPS** - 所有通信均使用 TLS 加密

## 🤝 贡献

欢迎贡献！请遵循以下指南：

1. Fork 仓库
2. 创建特性分支 (`git checkout -b feature/amazing-feature`)
3. 遵循 Rust 最佳实践和项目约定
4. 为新功能添加测试
5. 确保所有测试通过 (`cargo test`)
6. 运行格式化和 linting (`cargo fmt && cargo clippy`)
7. 提交你的更改 (`git commit -m 'Add amazing feature'`)
8. 推送到分支 (`git push origin feature/amazing-feature`)
9. 开启 Pull Request

### 开发约定

- **代码风格**: Rust 2024 edition, 100 字符行宽
- **测试**: 至少 80% 测试覆盖率
- **文档**: 所有公共 API 必须有文档
- **错误处理**: 使用 `thiserror` 用于自定义错误

## 📊 性能

专为高性能构建：
- **异步 I/O**: 使用 Tokio 的非阻塞操作
- **零拷贝解析**: 高效的 JSON 反序列化
- **连接池**: 复用 HTTP 连接
- **优化构建**: Release 版本启用 LTO 和单个 codegen 单元
- **基准测试**: 基于 Criterion 的性能基准测试

## 🐛 故障排除

### 常见问题

1. **编译错误**
   - 确保安装了 Rust 1.91+: `rustc --version`
   - 更新依赖: `cargo update`
   - 清理构建: `cargo clean && cargo build`

2. **API 认证失败**
   - 验证 `.env` 文件中的 API 密钥
   - 检查交易所上的 API 密钥权限
   - 确保系统时钟已同步

3. **速率限制**
   - 降低请求频率
   - 使用 WebSocket 获取实时数据
   - 检查交易所特定的速率限制

如需更多帮助，请参阅 [文档](docs/) 或开启 issue。

## 📝 许可证

本项目采用 MIT 许可证 - 详情请参阅 [LICENSE](LICENSE) 文件。

## 🙏 致谢

- 灵感来自原始 [CCXT](https://github.com/ccxt/ccxt) 库
- 基于出色的 Rust 生态系统库构建
- 社区贡献者和测试者

## 📞 联系与支持

- **Issues**: [GitHub Issues](https://github.com/Praying/ccxt-rust/issues)
- **Discussions**: [GitHub Discussions](https://github.com/Praying/ccxt-rust/discussions)
- **文档**: [项目文档](docs/)

---

**状态**: 🚧 积极开发中 | **版本**: 0.1.1 | **更新时间**: 2025-12

⚠️ **注意**: 本库正处于积极开发阶段。API 在 v1.0 之前可能会发生变化。暂不建议用于生产环境。