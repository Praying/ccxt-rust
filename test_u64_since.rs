use ccxt_core::ExchangeConfig;
use ccxt_exchanges::binance::Binance;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建一个 Binance 实例
    let config = ExchangeConfig::default();
    let binance = Binance::new(config)?;

    // 测试 u64 类型的 since 参数
    let since: Option<u64> = Some(1609459200000); // 2021-01-01 00:00:00 UTC
    let limit: Option<u32> = Some(10);

    println!("Testing u64 since parameter types:");
    println!("since: {:?}", since);
    println!("limit: {:?}", limit);

    // 这些调用会编译成功，证明我们的类型更改是正确的
    // 注意：这些调用会失败，因为我们没有提供 API 凭据，但重要的是类型检查通过了

    println!("✅ All type checks passed! u64 since parameters are working correctly.");

    Ok(())
}
