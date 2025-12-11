//! Binance WebSocket实现
//!
//! 提供Binance交易所的WebSocket实时数据流订阅功能

use crate::binance::Binance;
use crate::binance::parser;
use ccxt_core::error::{Error, Result};
use ccxt_core::types::financial::{Amount, Cost, Price};
use ccxt_core::types::{
    Balance, BidAsk, MarkPrice, MarketType, OHLCV, Order, OrderBook, Position, Ticker, Trade,
};
use ccxt_core::ws_client::{WsClient, WsConfig};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::protocol::Message;

/// Binance WebSocket端点
#[allow(dead_code)]
const WS_BASE_URL: &str = "wss://stream.binance.com:9443/ws";
#[allow(dead_code)]
const WS_TESTNET_URL: &str = "wss://testnet.binance.vision/ws";

/// Listen Key刷新间隔（30分钟）
const LISTEN_KEY_REFRESH_INTERVAL: Duration = Duration::from_secs(30 * 60);

/// Listen Key管理器
///
/// 自动管理Binance用户数据流的listen key，包括：
/// - 创建和缓存listen key
/// - 自动刷新（每30分钟）
/// - 过期检测和重建
/// - 连接状态管理
pub struct ListenKeyManager {
    /// Binance实例引用
    binance: Arc<Binance>,
    /// 当前的listen key
    listen_key: Arc<RwLock<Option<String>>>,
    /// listen key创建时间
    created_at: Arc<RwLock<Option<Instant>>>,
    /// 刷新间隔
    refresh_interval: Duration,
    /// 自动刷新任务句柄
    refresh_task: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl ListenKeyManager {
    /// 创建新的ListenKeyManager
    ///
    /// # Arguments
    /// * `binance` - Binance实例引用
    ///
    /// # Returns
    /// ListenKeyManager实例
    pub fn new(binance: Arc<Binance>) -> Self {
        Self {
            binance,
            listen_key: Arc::new(RwLock::new(None)),
            created_at: Arc::new(RwLock::new(None)),
            refresh_interval: LISTEN_KEY_REFRESH_INTERVAL,
            refresh_task: Arc::new(Mutex::new(None)),
        }
    }

    /// 获取或创建listen key
    ///
    /// 如果已有有效的listen key则返回，否则创建新的
    ///
    /// # Returns
    /// listen key字符串
    ///
    /// # Errors
    /// - 创建listen key失败
    /// - 缺少API认证信息
    pub async fn get_or_create(&self) -> Result<String> {
        // 检查是否已有listen key
        let key_opt = self.listen_key.read().await.clone();

        if let Some(key) = key_opt {
            // 检查是否需要刷新
            let created = self.created_at.read().await;
            if let Some(created_time) = *created {
                let elapsed = created_time.elapsed();
                // 如果已经超过50分钟，重新创建
                if elapsed > Duration::from_secs(50 * 60) {
                    drop(created);
                    return self.create_new().await;
                }
            }
            return Ok(key);
        }

        // 创建新的listen key
        self.create_new().await
    }

    /// 创建新的listen key
    ///
    /// # Returns
    /// 新创建的listen key
    async fn create_new(&self) -> Result<String> {
        let key = self.binance.create_listen_key().await?;

        // 更新缓存
        *self.listen_key.write().await = Some(key.clone());
        *self.created_at.write().await = Some(Instant::now());

        Ok(key)
    }

    /// 刷新listen key
    ///
    /// 延长当前listen key的有效期60分钟
    ///
    /// # Returns
    /// 刷新结果
    pub async fn refresh(&self) -> Result<()> {
        let key_opt = self.listen_key.read().await.clone();

        if let Some(key) = key_opt {
            self.binance.refresh_listen_key(&key).await?;
            // 更新创建时间
            *self.created_at.write().await = Some(Instant::now());
            Ok(())
        } else {
            Err(Error::invalid_request("No listen key to refresh"))
        }
    }

    /// 启动自动刷新任务
    ///
    /// 每30分钟自动刷新一次listen key
    pub async fn start_auto_refresh(&self) {
        // 停止现有任务
        self.stop_auto_refresh().await;

        let listen_key = self.listen_key.clone();
        let created_at = self.created_at.clone();
        let binance = self.binance.clone();
        let interval = self.refresh_interval;

        let handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;

                // 检查是否有listen key
                let key_opt = listen_key.read().await.clone();
                if let Some(key) = key_opt {
                    // 尝试刷新
                    match binance.refresh_listen_key(&key).await {
                        Ok(_) => {
                            *created_at.write().await = Some(Instant::now());
                            // Listen key refreshed successfully
                        }
                        Err(_e) => {
                            // Failed to refresh listen key, clear cache and recreate next time
                            *listen_key.write().await = None;
                            *created_at.write().await = None;
                            break;
                        }
                    }
                } else {
                    // 没有listen key，停止刷新任务
                    break;
                }
            }
        });

        *self.refresh_task.lock().await = Some(handle);
    }

    /// 停止自动刷新任务
    pub async fn stop_auto_refresh(&self) {
        let mut task_opt = self.refresh_task.lock().await;
        if let Some(handle) = task_opt.take() {
            handle.abort();
        }
    }

    /// 删除listen key
    ///
    /// 关闭用户数据流并使key失效
    ///
    /// # Returns
    /// 删除结果
    pub async fn delete(&self) -> Result<()> {
        // 停止自动刷新
        self.stop_auto_refresh().await;

        let key_opt = self.listen_key.read().await.clone();

        if let Some(key) = key_opt {
            self.binance.delete_listen_key(&key).await?;

            // 清除缓存
            *self.listen_key.write().await = None;
            *self.created_at.write().await = None;

            Ok(())
        } else {
            Ok(()) // 没有listen key，视为成功
        }
    }

    /// 获取当前listen key（如果存在）
    pub async fn get_current(&self) -> Option<String> {
        self.listen_key.read().await.clone()
    }

    /// 检查listen key是否有效
    pub async fn is_valid(&self) -> bool {
        let key_opt = self.listen_key.read().await;
        if key_opt.is_none() {
            return false;
        }

        let created = self.created_at.read().await;
        if let Some(created_time) = *created {
            // 检查是否在有效期内（小于55分钟）
            created_time.elapsed() < Duration::from_secs(55 * 60)
        } else {
            false
        }
    }
}

impl Drop for ListenKeyManager {
    fn drop(&mut self) {
        // 注意：由于Drop是同步的，我们不能在这里await异步操作
        // 用户应该显式调用delete()方法来清理资源
    }
}
/// 订阅类型
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SubscriptionType {
    /// 24小时行情ticker
    Ticker,
    /// 订单簿深度
    OrderBook,
    /// 实时成交
    Trades,
    /// K线数据（包含时间周期，如"1m", "5m", "1h"等）
    Kline(String),
    /// 账户余额
    Balance,
    /// 订单更新
    Orders,
    /// 持仓更新
    Positions,
    /// 我的成交记录
    MyTrades,
    /// 标记价格
    MarkPrice,
    /// 最优挂单
    BookTicker,
}

impl SubscriptionType {
    /// 从流名称识别订阅类型
    ///
    /// # Arguments
    /// * `stream` - 流名称（如"btcusdt@ticker"）
    ///
    /// # Returns
    /// 订阅类型（如果能识别）
    pub fn from_stream(stream: &str) -> Option<Self> {
        if stream.contains("@ticker") {
            Some(Self::Ticker)
        } else if stream.contains("@depth") {
            Some(Self::OrderBook)
        } else if stream.contains("@trade") || stream.contains("@aggTrade") {
            Some(Self::Trades)
        } else if stream.contains("@kline_") {
            // 提取时间周期
            let parts: Vec<&str> = stream.split("@kline_").collect();
            if parts.len() == 2 {
                Some(Self::Kline(parts[1].to_string()))
            } else {
                None
            }
        } else if stream.contains("@markPrice") {
            Some(Self::MarkPrice)
        } else if stream.contains("@bookTicker") {
            Some(Self::BookTicker)
        } else {
            None
        }
    }
}

/// 订阅信息
#[derive(Clone)]
pub struct Subscription {
    /// 订阅的流名称（如 "btcusdt@ticker"）
    pub stream: String,
    /// 交易对符号（如 "BTCUSDT"，标准化格式）
    pub symbol: String,
    /// 订阅类型
    pub sub_type: SubscriptionType,
    /// 订阅时间
    pub subscribed_at: Instant,
    /// 消息发送器（用于发送数据到消费者）
    pub sender: tokio::sync::mpsc::UnboundedSender<Value>,
}

impl Subscription {
    /// 创建新的订阅
    pub fn new(
        stream: String,
        symbol: String,
        sub_type: SubscriptionType,
        sender: tokio::sync::mpsc::UnboundedSender<Value>,
    ) -> Self {
        Self {
            stream,
            symbol,
            sub_type,
            subscribed_at: Instant::now(),
            sender,
        }
    }

    /// 发送消息到订阅者
    ///
    /// # Arguments
    /// * `message` - WebSocket消息
    ///
    /// # Returns
    /// 发送是否成功
    pub fn send(&self, message: Value) -> bool {
        self.sender.send(message).is_ok()
    }
}

/// 订阅管理器
///
/// 负责管理所有WebSocket订阅的生命周期，包括：
/// - 添加和移除订阅
/// - 订阅查询和验证
/// - 统计活跃订阅数
pub struct SubscriptionManager {
    /// 订阅映射：stream_name -> Subscription
    subscriptions: Arc<RwLock<HashMap<String, Subscription>>>,
    /// 按交易对索引：symbol -> Vec<stream_name>
    symbol_index: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// 活跃订阅计数
    active_count: Arc<std::sync::atomic::AtomicUsize>,
}

impl SubscriptionManager {
    /// 创建新的订阅管理器
    pub fn new() -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            symbol_index: Arc::new(RwLock::new(HashMap::new())),
            active_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    /// 添加订阅
    ///
    /// # Arguments
    /// * `stream` - 流名称（如"btcusdt@ticker"）
    /// * `symbol` - 交易对符号（标准化格式，如"BTCUSDT"）
    /// * `sub_type` - 订阅类型
    /// * `sender` - 消息发送器
    ///
    /// # Returns
    /// 添加结果
    pub async fn add_subscription(
        &self,
        stream: String,
        symbol: String,
        sub_type: SubscriptionType,
        sender: tokio::sync::mpsc::UnboundedSender<Value>,
    ) -> Result<()> {
        let subscription = Subscription::new(stream.clone(), symbol.clone(), sub_type, sender);

        // 添加到订阅映射
        let mut subs = self.subscriptions.write().await;
        subs.insert(stream.clone(), subscription);

        // 更新symbol索引
        let mut index = self.symbol_index.write().await;
        index.entry(symbol).or_insert_with(Vec::new).push(stream);

        // 更新计数
        self.active_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        Ok(())
    }

    /// 移除订阅
    ///
    /// # Arguments
    /// * `stream` - 流名称
    ///
    /// # Returns
    /// 移除结果
    pub async fn remove_subscription(&self, stream: &str) -> Result<()> {
        let mut subs = self.subscriptions.write().await;

        if let Some(subscription) = subs.remove(stream) {
            // 从symbol索引中移除
            let mut index = self.symbol_index.write().await;
            if let Some(streams) = index.get_mut(&subscription.symbol) {
                streams.retain(|s| s != stream);
                // 如果该symbol没有订阅了，移除整个条目
                if streams.is_empty() {
                    index.remove(&subscription.symbol);
                }
            }

            // 更新计数
            self.active_count
                .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        }

        Ok(())
    }

    /// 获取订阅
    ///
    /// # Arguments
    /// * `stream` - 流名称
    ///
    /// # Returns
    /// 订阅信息（如果存在）
    pub async fn get_subscription(&self, stream: &str) -> Option<Subscription> {
        let subs = self.subscriptions.read().await;
        subs.get(stream).cloned()
    }

    /// 检查订阅是否存在
    ///
    /// # Arguments
    /// * `stream` - 流名称
    ///
    /// # Returns
    /// 是否存在
    pub async fn has_subscription(&self, stream: &str) -> bool {
        let subs = self.subscriptions.read().await;
        subs.contains_key(stream)
    }

    /// 获取所有订阅
    ///
    /// # Returns
    /// 订阅列表
    pub async fn get_all_subscriptions(&self) -> Vec<Subscription> {
        let subs = self.subscriptions.read().await;
        subs.values().cloned().collect()
    }

    /// 获取指定交易对的所有订阅
    ///
    /// # Arguments
    /// * `symbol` - 交易对符号
    ///
    /// # Returns
    /// 订阅列表
    pub async fn get_subscriptions_by_symbol(&self, symbol: &str) -> Vec<Subscription> {
        let index = self.symbol_index.read().await;
        let subs = self.subscriptions.read().await;

        if let Some(streams) = index.get(symbol) {
            streams
                .iter()
                .filter_map(|stream| subs.get(stream).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// 获取活跃订阅数
    ///
    /// # Returns
    /// 活跃订阅数量
    pub fn active_count(&self) -> usize {
        self.active_count.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// 清除所有订阅
    pub async fn clear(&self) {
        let mut subs = self.subscriptions.write().await;
        let mut index = self.symbol_index.write().await;

        subs.clear();
        index.clear();
        self.active_count
            .store(0, std::sync::atomic::Ordering::SeqCst);
    }

    /// 向指定流的订阅者发送消息
    ///
    /// # Arguments
    /// * `stream` - 流名称
    /// * `message` - WebSocket消息
    ///
    /// # Returns
    /// 是否发送成功
    pub async fn send_to_stream(&self, stream: &str, message: Value) -> bool {
        let subs = self.subscriptions.read().await;
        if let Some(subscription) = subs.get(stream) {
            subscription.send(message)
        } else {
            false
        }
    }

    /// 向指定交易对的所有订阅者发送消息
    ///
    /// # Arguments
    /// * `symbol` - 交易对符号
    /// * `message` - WebSocket消息
    ///
    /// # Returns
    /// 成功发送的订阅数
    pub async fn send_to_symbol(&self, symbol: &str, message: &Value) -> usize {
        let index = self.symbol_index.read().await;
        let subs = self.subscriptions.read().await;

        let mut sent_count = 0;

        if let Some(streams) = index.get(symbol) {
            for stream in streams {
                if let Some(subscription) = subs.get(stream) {
                    if subscription.send(message.clone()) {
                        sent_count += 1;
                    }
                }
            }
        }

        sent_count
    }
}
/// 重连配置
///
/// 配置WebSocket连接断开后的自动重连策略
#[derive(Debug, Clone)]
pub struct ReconnectConfig {
    /// 是否启用自动重连
    pub enabled: bool,

    /// 初始重连延迟（毫秒）
    pub initial_delay_ms: u64,

    /// 最大重连延迟（毫秒）
    pub max_delay_ms: u64,

    /// 延迟倍增因子
    pub backoff_multiplier: f64,

    /// 最大重连尝试次数（0表示无限制）
    pub max_attempts: usize,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            initial_delay_ms: 1000,  // 1秒
            max_delay_ms: 30000,     // 30秒
            backoff_multiplier: 2.0, // 指数退避
            max_attempts: 0,         // 无限重连
        }
    }
}

impl ReconnectConfig {
    /// 计算重连延迟
    ///
    /// 使用指数退避策略计算延迟时间
    ///
    /// # Arguments
    /// * `attempt` - 当前重连尝试次数（从0开始）
    ///
    /// # Returns
    /// 延迟时间（毫秒）
    pub fn calculate_delay(&self, attempt: usize) -> u64 {
        let delay = (self.initial_delay_ms as f64) * self.backoff_multiplier.powi(attempt as i32);
        delay.min(self.max_delay_ms as f64) as u64
    }

    /// 检查是否应该继续重连
    ///
    /// # Arguments
    /// * `attempt` - 当前重连尝试次数
    ///
    /// # Returns
    /// 是否应该继续重连
    pub fn should_retry(&self, attempt: usize) -> bool {
        self.enabled && (self.max_attempts == 0 || attempt < self.max_attempts)
    }
}

/// 消息路由器
///
/// 负责WebSocket消息的接收、解析和分发，核心功能包括：
/// - WebSocket连接管理
/// - 消息接收和路由
/// - 自动重连机制
/// - 订阅管理
pub struct MessageRouter {
    /// WebSocket客户端
    ws_client: Arc<RwLock<Option<WsClient>>>,

    /// 订阅管理器
    subscription_manager: Arc<SubscriptionManager>,

    /// 路由任务句柄
    router_task: Arc<Mutex<Option<JoinHandle<()>>>>,

    /// 连接状态标志
    is_connected: Arc<std::sync::atomic::AtomicBool>,

    /// 重连配置
    reconnect_config: Arc<RwLock<ReconnectConfig>>,

    /// WebSocket URL
    ws_url: String,

    /// 请求ID计数器（用于订阅/取消订阅）
    request_id: Arc<std::sync::atomic::AtomicU64>,
}

impl MessageRouter {
    /// 创建新的消息路由器
    ///
    /// # Arguments
    /// * `ws_url` - WebSocket连接URL
    /// * `subscription_manager` - 订阅管理器
    ///
    /// # Returns
    /// MessageRouter实例
    pub fn new(ws_url: String, subscription_manager: Arc<SubscriptionManager>) -> Self {
        Self {
            ws_client: Arc::new(RwLock::new(None)),
            subscription_manager,
            router_task: Arc::new(Mutex::new(None)),
            is_connected: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            reconnect_config: Arc::new(RwLock::new(ReconnectConfig::default())),
            ws_url,
            request_id: Arc::new(std::sync::atomic::AtomicU64::new(1)),
        }
    }

    /// 启动消息路由器
    ///
    /// 建立WebSocket连接并启动消息接收循环
    ///
    /// # Returns
    /// 启动结果
    pub async fn start(&self) -> Result<()> {
        // 如果已经在运行，先停止
        if self.is_connected() {
            self.stop().await?;
        }

        // 建立WebSocket连接
        let config = WsConfig {
            url: self.ws_url.clone(),
            ..Default::default()
        };
        let client = WsClient::new(config);
        client.connect().await?;

        // 保存客户端
        *self.ws_client.write().await = Some(client);

        // 设置连接状态
        self.is_connected
            .store(true, std::sync::atomic::Ordering::SeqCst);

        // 启动消息循环
        let ws_client = self.ws_client.clone();
        let subscription_manager = self.subscription_manager.clone();
        let is_connected = self.is_connected.clone();
        let reconnect_config = self.reconnect_config.clone();
        let ws_url = self.ws_url.clone();

        let handle = tokio::spawn(async move {
            Self::message_loop(
                ws_client,
                subscription_manager,
                is_connected,
                reconnect_config,
                ws_url,
            )
            .await
        });

        *self.router_task.lock().await = Some(handle);

        Ok(())
    }

    /// 停止消息路由器
    ///
    /// 停止消息接收任务并关闭WebSocket连接
    ///
    /// # Returns
    /// 停止结果
    pub async fn stop(&self) -> Result<()> {
        // 设置连接状态为false
        self.is_connected
            .store(false, std::sync::atomic::Ordering::SeqCst);

        // 停止路由任务
        let mut task_opt = self.router_task.lock().await;
        if let Some(handle) = task_opt.take() {
            handle.abort();
        }

        // 关闭WebSocket连接
        let mut client_opt = self.ws_client.write().await;
        if let Some(client) = client_opt.take() {
            let _ = client.disconnect().await;
        }

        Ok(())
    }

    /// 重启消息路由器
    ///
    /// 先停止再启动，用于重连场景
    ///
    /// # Returns
    /// 重启结果
    pub async fn restart(&self) -> Result<()> {
        self.stop().await?;
        tokio::time::sleep(Duration::from_millis(100)).await;
        self.start().await
    }

    /// 获取连接状态
    ///
    /// # Returns
    /// 是否已连接
    pub fn is_connected(&self) -> bool {
        self.is_connected.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// 设置重连配置
    ///
    /// # Arguments
    /// * `config` - 重连配置
    pub async fn set_reconnect_config(&self, config: ReconnectConfig) {
        *self.reconnect_config.write().await = config;
    }

    /// 获取重连配置
    ///
    /// # Returns
    /// 当前重连配置
    pub async fn get_reconnect_config(&self) -> ReconnectConfig {
        self.reconnect_config.read().await.clone()
    }

    /// 订阅流
    ///
    /// 向Binance发送订阅请求
    ///
    /// # Arguments
    /// * `streams` - 流名称列表
    ///
    /// # Returns
    /// 订阅结果
    pub async fn subscribe(&self, streams: Vec<String>) -> Result<()> {
        if streams.is_empty() {
            return Ok(());
        }

        let client_opt = self.ws_client.read().await;
        let client = client_opt
            .as_ref()
            .ok_or_else(|| Error::network("WebSocket not connected"))?;

        // 生成请求ID
        let id = self
            .request_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        // 构造订阅请求
        // json! macro with literal values is infallible
        #[allow(clippy::disallowed_methods)]
        let request = serde_json::json!({
            "method": "SUBSCRIBE",
            "params": streams,
            "id": id
        });

        // 发送订阅请求
        client
            .send(Message::Text(request.to_string().into()))
            .await?;

        Ok(())
    }

    /// 取消订阅流
    ///
    /// 向Binance发送取消订阅请求
    ///
    /// # Arguments
    /// * `streams` - 流名称列表
    ///
    /// # Returns
    /// 取消订阅结果
    pub async fn unsubscribe(&self, streams: Vec<String>) -> Result<()> {
        if streams.is_empty() {
            return Ok(());
        }

        let client_opt = self.ws_client.read().await;
        let client = client_opt
            .as_ref()
            .ok_or_else(|| Error::network("WebSocket not connected"))?;

        // 生成请求ID
        let id = self
            .request_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        // 构造取消订阅请求
        // json! macro with literal values is infallible
        #[allow(clippy::disallowed_methods)]
        let request = serde_json::json!({
            "method": "UNSUBSCRIBE",
            "params": streams,
            "id": id
        });

        // 发送取消订阅请求
        client
            .send(Message::Text(request.to_string().into()))
            .await?;

        Ok(())
    }

    /// 消息接收循环
    ///
    /// 持续接收WebSocket消息并路由到订阅者
    async fn message_loop(
        ws_client: Arc<RwLock<Option<WsClient>>>,
        subscription_manager: Arc<SubscriptionManager>,
        is_connected: Arc<std::sync::atomic::AtomicBool>,
        reconnect_config: Arc<RwLock<ReconnectConfig>>,
        ws_url: String,
    ) {
        let mut reconnect_attempt = 0;

        loop {
            // 检查是否应该停止
            if !is_connected.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }

            // 检查客户端是否存在
            let has_client = ws_client.read().await.is_some();

            if !has_client {
                // 尝试重连
                let config = reconnect_config.read().await;
                if config.should_retry(reconnect_attempt) {
                    let delay = config.calculate_delay(reconnect_attempt);
                    drop(config);

                    tokio::time::sleep(Duration::from_millis(delay)).await;

                    match Self::reconnect(&ws_url, ws_client.clone()).await {
                        Ok(_) => {
                            reconnect_attempt = 0; // 重连成功，重置计数
                            continue;
                        }
                        Err(_) => {
                            reconnect_attempt += 1;
                            continue;
                        }
                    }
                } else {
                    // 不再重连，退出循环
                    is_connected.store(false, std::sync::atomic::Ordering::SeqCst);
                    break;
                }
            }

            // 接收消息（需要重新获取锁）
            let message_opt = {
                let guard = ws_client.read().await;
                if let Some(client) = guard.as_ref() {
                    client.receive().await
                } else {
                    None
                }
            };

            match message_opt {
                Some(value) => {
                    // 处理消息
                    if let Err(_e) = Self::handle_message(value, subscription_manager.clone()).await
                    {
                        // 消息处理错误，继续接收下一条
                        continue;
                    }

                    // 重置重连计数（连接正常）
                    reconnect_attempt = 0;
                }
                None => {
                    // 连接错误，尝试重连
                    let config = reconnect_config.read().await;
                    if config.should_retry(reconnect_attempt) {
                        let delay = config.calculate_delay(reconnect_attempt);
                        drop(config);

                        tokio::time::sleep(Duration::from_millis(delay)).await;

                        match Self::reconnect(&ws_url, ws_client.clone()).await {
                            Ok(_) => {
                                reconnect_attempt = 0;
                                continue;
                            }
                            Err(_) => {
                                reconnect_attempt += 1;
                                continue;
                            }
                        }
                    } else {
                        // 不再重连，退出循环
                        is_connected.store(false, std::sync::atomic::Ordering::SeqCst);
                        break;
                    }
                }
            }
        }
    }

    /// 处理WebSocket消息
    ///
    /// 解析消息并路由到对应订阅者
    async fn handle_message(
        message: Value,
        subscription_manager: Arc<SubscriptionManager>,
    ) -> Result<()> {
        // 提取流名称
        let stream_name = Self::extract_stream_name(&message)?;

        // 路由消息到订阅者
        let sent = subscription_manager
            .send_to_stream(&stream_name, message)
            .await;

        if sent {
            Ok(())
        } else {
            Err(Error::generic("No subscribers for stream"))
        }
    }

    /// 从消息中提取流名称
    ///
    /// 支持两种消息格式：
    /// 1. 组合流：`{"stream":"btcusdt@ticker","data":{...}}`
    /// 2. 单流：`{"e":"24hrTicker","s":"BTCUSDT",...}`
    ///
    /// # Arguments
    /// * `message` - WebSocket消息
    ///
    /// # Returns
    /// 流名称
    fn extract_stream_name(message: &Value) -> Result<String> {
        // 尝试从组合流格式提取
        if let Some(stream) = message.get("stream").and_then(|s| s.as_str()) {
            return Ok(stream.to_string());
        }

        // 尝试从单流格式提取
        // 格式：事件类型@交易对（小写）
        if let Some(event_type) = message.get("e").and_then(|e| e.as_str()) {
            if let Some(symbol) = message.get("s").and_then(|s| s.as_str()) {
                // 构造流名称
                let stream = match event_type {
                    "24hrTicker" => format!("{}@ticker", symbol.to_lowercase()),
                    "depthUpdate" => format!("{}@depth", symbol.to_lowercase()),
                    "aggTrade" => format!("{}@aggTrade", symbol.to_lowercase()),
                    "trade" => format!("{}@trade", symbol.to_lowercase()),
                    "kline" => {
                        // K线需要提取时间周期
                        if let Some(kline) = message.get("k") {
                            if let Some(interval) = kline.get("i").and_then(|i| i.as_str()) {
                                format!("{}@kline_{}", symbol.to_lowercase(), interval)
                            } else {
                                return Err(Error::generic("Missing kline interval"));
                            }
                        } else {
                            return Err(Error::generic("Missing kline data"));
                        }
                    }
                    "markPriceUpdate" => format!("{}@markPrice", symbol.to_lowercase()),
                    "bookTicker" => format!("{}@bookTicker", symbol.to_lowercase()),
                    _ => {
                        return Err(Error::generic(format!(
                            "Unknown event type: {}",
                            event_type
                        )));
                    }
                };
                return Ok(stream);
            }
        }

        // 如果是订阅响应或错误响应，不需要路由
        if message.get("result").is_some() || message.get("error").is_some() {
            return Err(Error::generic("Subscription response, skip routing"));
        }

        Err(Error::generic("Cannot extract stream name from message"))
    }

    /// 重连WebSocket
    ///
    /// 关闭旧连接并建立新连接
    async fn reconnect(ws_url: &str, ws_client: Arc<RwLock<Option<WsClient>>>) -> Result<()> {
        // 关闭旧连接
        {
            let mut client_opt = ws_client.write().await;
            if let Some(client) = client_opt.take() {
                let _ = client.disconnect().await;
            }
        }

        // 建立新连接
        let config = WsConfig {
            url: ws_url.to_string(),
            ..Default::default()
        };
        let new_client = WsClient::new(config);

        // 连接WebSocket
        new_client.connect().await?;

        // 保存新客户端
        *ws_client.write().await = Some(new_client);

        Ok(())
    }
}

impl Drop for MessageRouter {
    fn drop(&mut self) {
        // 注意：由于Drop是同步的，我们不能在这里await异步操作
        // 用户应该显式调用stop()方法来清理资源
    }
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Binance WebSocket客户端包装器
pub struct BinanceWs {
    client: Arc<WsClient>,
    listen_key: Arc<RwLock<Option<String>>>,
    /// Listen key管理器
    listen_key_manager: Option<Arc<ListenKeyManager>>,
    /// 自动重连协调器
    auto_reconnect_coordinator: Arc<Mutex<Option<ccxt_core::ws_client::AutoReconnectCoordinator>>>,
    /// 缓存的ticker数据
    tickers: Arc<Mutex<HashMap<String, Ticker>>>,
    /// 缓存的bid/ask数据
    bids_asks: Arc<Mutex<HashMap<String, BidAsk>>>,
    /// 缓存的mark price数据
    #[allow(dead_code)]
    mark_prices: Arc<Mutex<HashMap<String, MarkPrice>>>,
    /// 缓存的订单簿数据
    orderbooks: Arc<Mutex<HashMap<String, OrderBook>>>,
    /// 缓存的交易数据（保留最近1000条）
    trades: Arc<Mutex<HashMap<String, VecDeque<Trade>>>>,
    /// 缓存的K线数据
    ohlcvs: Arc<Mutex<HashMap<String, VecDeque<OHLCV>>>>,
    /// 缓存的余额数据（按账户类型）
    balances: Arc<RwLock<HashMap<String, Balance>>>,
    /// 缓存的订单数据（按symbol分组，再按orderId索引）
    orders: Arc<RwLock<HashMap<String, HashMap<String, Order>>>>,
    /// 缓存的成交记录（按symbol分组，保留最近1000条）
    my_trades: Arc<RwLock<HashMap<String, VecDeque<Trade>>>>,
    /// 缓存的持仓数据（按symbol和side分组）
    positions: Arc<RwLock<HashMap<String, HashMap<String, Position>>>>,
}

impl BinanceWs {
    /// 创建新的Binance WebSocket客户端
    ///
    /// # Arguments
    /// * `url` - WebSocket服务器URL
    ///
    /// # Returns
    /// Binance WebSocket客户端实例
    pub fn new(url: String) -> Self {
        let config = WsConfig {
            url,
            connect_timeout: 10000,
            ping_interval: 180000, // Binance推荐3分钟
            reconnect_interval: 5000,
            max_reconnect_attempts: 5,
            auto_reconnect: true,
            enable_compression: false,
            pong_timeout: 90000, // pong超时设置为90秒
        };

        Self {
            client: Arc::new(WsClient::new(config)),
            listen_key: Arc::new(RwLock::new(None)),
            listen_key_manager: None,
            auto_reconnect_coordinator: Arc::new(Mutex::new(None)),
            tickers: Arc::new(Mutex::new(HashMap::new())),
            bids_asks: Arc::new(Mutex::new(HashMap::new())),
            mark_prices: Arc::new(Mutex::new(HashMap::new())),
            orderbooks: Arc::new(Mutex::new(HashMap::new())),
            trades: Arc::new(Mutex::new(HashMap::new())),
            ohlcvs: Arc::new(Mutex::new(HashMap::new())),
            balances: Arc::new(RwLock::new(HashMap::new())),
            orders: Arc::new(RwLock::new(HashMap::new())),
            my_trades: Arc::new(RwLock::new(HashMap::new())),
            positions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 创建带有listen key管理器的WebSocket客户端
    ///
    /// # Arguments
    /// * `url` - WebSocket服务器URL
    /// * `binance` - Binance实例引用
    ///
    /// # Returns
    /// Binance WebSocket客户端实例（带listen key管理器）
    pub fn new_with_auth(url: String, binance: Arc<Binance>) -> Self {
        let config = WsConfig {
            url,
            connect_timeout: 10000,
            ping_interval: 180000,
            reconnect_interval: 5000,
            max_reconnect_attempts: 5,
            auto_reconnect: true,
            enable_compression: false,
            pong_timeout: 90000, // pong超时设置为90秒
        };

        Self {
            client: Arc::new(WsClient::new(config)),
            listen_key: Arc::new(RwLock::new(None)),
            listen_key_manager: Some(Arc::new(ListenKeyManager::new(binance))),
            auto_reconnect_coordinator: Arc::new(Mutex::new(None)),
            tickers: Arc::new(Mutex::new(HashMap::new())),
            bids_asks: Arc::new(Mutex::new(HashMap::new())),
            mark_prices: Arc::new(Mutex::new(HashMap::new())),
            orderbooks: Arc::new(Mutex::new(HashMap::new())),
            trades: Arc::new(Mutex::new(HashMap::new())),
            ohlcvs: Arc::new(Mutex::new(HashMap::new())),
            balances: Arc::new(RwLock::new(HashMap::new())),
            orders: Arc::new(RwLock::new(HashMap::new())),
            my_trades: Arc::new(RwLock::new(HashMap::new())),
            positions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 连接到WebSocket服务器
    pub async fn connect(&self) -> Result<()> {
        // 先连接WebSocket
        self.client.connect().await?;

        // 启动自动重连协调器
        let mut coordinator_guard = self.auto_reconnect_coordinator.lock().await;
        if coordinator_guard.is_none() {
            let coordinator = self.client.clone().create_auto_reconnect_coordinator();
            coordinator.start().await;
            *coordinator_guard = Some(coordinator);
            tracing::info!("Auto-reconnect coordinator started");
        }

        Ok(())
    }

    /// 断开WebSocket连接
    pub async fn disconnect(&self) -> Result<()> {
        // 停止自动重连协调器
        let mut coordinator_guard = self.auto_reconnect_coordinator.lock().await;
        if let Some(coordinator) = coordinator_guard.take() {
            coordinator.stop().await;
            tracing::info!("Auto-reconnect coordinator stopped");
        }

        // 如果有listen key管理器，停止自动刷新
        if let Some(manager) = &self.listen_key_manager {
            manager.stop_auto_refresh().await;
        }

        self.client.disconnect().await
    }

    /// 连接用户数据流
    ///
    /// 创建或获取listen key，连接到用户数据流WebSocket，并启动自动刷新
    ///
    /// # Returns
    /// 连接结果
    ///
    /// # Errors
    /// - 缺少listen key管理器（需要使用new_with_auth创建实例）
    /// - 创建listen key失败
    /// - WebSocket连接失败
    ///
    /// # Example
    /// ```rust,no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use std::sync::Arc;
    /// # async fn example() -> ccxt_core::error::Result<()> {
    /// let binance = Arc::new(Binance::new());
    /// let ws = binance.create_authenticated_ws();
    /// ws.connect_user_stream().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect_user_stream(&self) -> Result<()> {
        let manager = self.listen_key_manager.as_ref()
            .ok_or_else(|| Error::invalid_request(
                "Listen key manager not available. Use new_with_auth() to create authenticated WebSocket"
            ))?;

        // 获取或创建listen key
        let listen_key = manager.get_or_create().await?;

        // 更新配置中的URL
        let user_stream_url = format!("wss://stream.binance.com:9443/ws/{}", listen_key);

        // 重新创建WebSocket客户端
        let config = WsConfig {
            url: user_stream_url,
            connect_timeout: 10000,
            ping_interval: 180000,
            reconnect_interval: 5000,
            max_reconnect_attempts: 5,
            auto_reconnect: true,
            enable_compression: false,
            pong_timeout: 90000, // 90秒，默认值
        };

        // 更新客户端配置
        // 注意：这里我们需要重新创建WsClient
        let _new_client = Arc::new(WsClient::new(config));
        // 由于client是Arc，我们不能直接修改，需要在实际使用时处理

        // 连接到WebSocket
        self.client.connect().await?;

        // 启动自动刷新
        manager.start_auto_refresh().await;

        // 缓存listen key
        *self.listen_key.write().await = Some(listen_key);

        Ok(())
    }

    /// 关闭用户数据流
    ///
    /// 停止自动刷新并删除listen key
    ///
    /// # Returns
    /// 关闭结果
    pub async fn close_user_stream(&self) -> Result<()> {
        if let Some(manager) = &self.listen_key_manager {
            manager.delete().await?;
        }
        *self.listen_key.write().await = None;
        Ok(())
    }

    /// 获取当前的listen key（如果有）
    pub async fn get_listen_key(&self) -> Option<String> {
        if let Some(manager) = &self.listen_key_manager {
            manager.get_current().await
        } else {
            self.listen_key.read().await.clone()
        }
    }

    /// 订阅行情ticker
    ///
    /// # Arguments
    /// * `symbol` - 交易对（如 "btcusdt"）
    ///
    /// # Returns
    /// 订阅结果
    pub async fn subscribe_ticker(&self, symbol: &str) -> Result<()> {
        let stream = format!("{}@ticker", symbol.to_lowercase());
        self.client
            .subscribe(stream, Some(symbol.to_string()), None)
            .await
    }

    /// 订阅24小时所有ticker
    pub async fn subscribe_all_tickers(&self) -> Result<()> {
        self.client
            .subscribe("!ticker@arr".to_string(), None, None)
            .await
    }

    /// 订阅交易记录
    ///
    /// # Arguments
    /// * `symbol` - 交易对（如 "btcusdt"）
    ///
    /// # Returns
    /// 订阅结果
    pub async fn subscribe_trades(&self, symbol: &str) -> Result<()> {
        let stream = format!("{}@trade", symbol.to_lowercase());
        self.client
            .subscribe(stream, Some(symbol.to_string()), None)
            .await
    }

    /// 订阅聚合交易记录
    ///
    /// # Arguments
    /// * `symbol` - 交易对（如 "btcusdt"）
    ///
    /// # Returns
    /// 订阅结果
    pub async fn subscribe_agg_trades(&self, symbol: &str) -> Result<()> {
        let stream = format!("{}@aggTrade", symbol.to_lowercase());
        self.client
            .subscribe(stream, Some(symbol.to_string()), None)
            .await
    }

    /// 订阅订单簿深度
    ///
    /// # Arguments
    /// * `symbol` - 交易对（如 "btcusdt"）
    /// * `levels` - 深度档位（5, 10, 20）
    /// * `update_speed` - 更新速度（"100ms" 或 "1000ms"）
    ///
    /// # Returns
    /// 订阅结果
    pub async fn subscribe_orderbook(
        &self,
        symbol: &str,
        levels: u32,
        update_speed: &str,
    ) -> Result<()> {
        let stream = if update_speed == "100ms" {
            format!("{}@depth{}@100ms", symbol.to_lowercase(), levels)
        } else {
            format!("{}@depth{}", symbol.to_lowercase(), levels)
        };

        self.client
            .subscribe(stream, Some(symbol.to_string()), None)
            .await
    }

    /// 订阅差分深度
    ///
    /// # Arguments
    /// * `symbol` - 交易对（如 "btcusdt"）
    /// * `update_speed` - 更新速度（"100ms" 或 "1000ms"）
    ///
    /// # Returns
    /// 订阅结果
    pub async fn subscribe_orderbook_diff(
        &self,
        symbol: &str,
        update_speed: Option<&str>,
    ) -> Result<()> {
        let stream = if let Some(speed) = update_speed {
            if speed == "100ms" {
                format!("{}@depth@100ms", symbol.to_lowercase())
            } else {
                format!("{}@depth", symbol.to_lowercase())
            }
        } else {
            format!("{}@depth", symbol.to_lowercase())
        };

        self.client
            .subscribe(stream, Some(symbol.to_string()), None)
            .await
    }

    /// 订阅K线数据
    ///
    /// # Arguments
    /// * `symbol` - 交易对（如 "btcusdt"）
    /// * `interval` - K线周期（1m, 3m, 5m, 15m, 30m, 1h, 2h, 4h, 6h, 8h, 12h, 1d, 3d, 1w, 1M）
    ///
    /// # Returns
    /// 订阅结果
    pub async fn subscribe_kline(&self, symbol: &str, interval: &str) -> Result<()> {
        let stream = format!("{}@kline_{}", symbol.to_lowercase(), interval);
        self.client
            .subscribe(stream, Some(symbol.to_string()), None)
            .await
    }

    /// 订阅迷你ticker
    ///
    /// # Arguments
    /// * `symbol` - 交易对（如 "btcusdt"）
    ///
    /// # Returns
    /// 订阅结果
    pub async fn subscribe_mini_ticker(&self, symbol: &str) -> Result<()> {
        let stream = format!("{}@miniTicker", symbol.to_lowercase());
        self.client
            .subscribe(stream, Some(symbol.to_string()), None)
            .await
    }

    /// 订阅所有迷你ticker
    pub async fn subscribe_all_mini_tickers(&self) -> Result<()> {
        self.client
            .subscribe("!miniTicker@arr".to_string(), None, None)
            .await
    }

    /// 取消订阅
    ///
    /// # Arguments
    /// * `stream` - 流名称
    ///
    /// # Returns
    /// 取消订阅结果
    pub async fn unsubscribe(&self, stream: String) -> Result<()> {
        self.client.unsubscribe(stream, None).await
    }

    /// 接收消息
    ///
    /// # Returns
    /// 接收到的消息（如果有）
    pub async fn receive(&self) -> Option<Value> {
        self.client.receive().await
    }

    /// 是否已连接
    pub async fn is_connected(&self) -> bool {
        self.client.is_connected().await
    }

    /// Watch单个ticker（内部方法）
    ///
    /// # Arguments
    /// * `symbol` - 交易对（小写格式，如 "btcusdt"）
    /// * `channel_name` - 频道名称（ticker/miniTicker/markPrice/bookTicker）
    ///
    /// # Returns
    /// Ticker数据
    async fn watch_ticker_internal(&self, symbol: &str, channel_name: &str) -> Result<Ticker> {
        let stream = format!("{}@{}", symbol.to_lowercase(), channel_name);

        // 订阅流
        self.client
            .subscribe(stream.clone(), Some(symbol.to_string()), None)
            .await?;

        // 等待并解析消息
        loop {
            if let Some(message) = self.client.receive().await {
                // 检查是否是订阅确认消息
                if message.get("result").is_some() {
                    continue;
                }

                // 解析ticker数据
                if let Ok(ticker) = parser::parse_ws_ticker(&message, None) {
                    // 缓存ticker
                    let mut tickers = self.tickers.lock().await;
                    tickers.insert(ticker.symbol.clone(), ticker.clone());

                    return Ok(ticker);
                }
            }
        }
    }

    /// Watch多个ticker（内部方法）
    ///
    /// # Arguments
    /// * `symbols` - 交易对列表（小写格式）
    /// * `channel_name` - 频道名称
    ///
    /// # Returns
    /// Ticker数据映射
    async fn watch_tickers_internal(
        &self,
        symbols: Option<Vec<String>>,
        channel_name: &str,
    ) -> Result<HashMap<String, Ticker>> {
        let streams: Vec<String> = if let Some(syms) = symbols.as_ref() {
            // 订阅指定交易对
            syms.iter()
                .map(|s| format!("{}@{}", s.to_lowercase(), channel_name))
                .collect()
        } else {
            // 订阅所有交易对
            vec![format!("!{}@arr", channel_name)]
        };

        // 批量订阅
        for stream in &streams {
            self.client.subscribe(stream.clone(), None, None).await?;
        }

        // 等待并解析消息
        let mut result = HashMap::new();

        loop {
            if let Some(message) = self.client.receive().await {
                // 检查是否是订阅确认消息
                if message.get("result").is_some() {
                    continue;
                }

                // 处理数组格式（所有ticker）
                if let Some(arr) = message.as_array() {
                    for item in arr {
                        if let Ok(ticker) = parser::parse_ws_ticker(item, None) {
                            let symbol = ticker.symbol.clone();

                            // 如果指定了symbols，只返回匹配的
                            if let Some(syms) = &symbols {
                                if syms.contains(&symbol.to_lowercase()) {
                                    result.insert(symbol.clone(), ticker.clone());
                                }
                            } else {
                                result.insert(symbol.clone(), ticker.clone());
                            }

                            // 缓存ticker
                            let mut tickers = self.tickers.lock().await;
                            tickers.insert(symbol, ticker);
                        }
                    }

                    // 如果已经收到了所有请求的ticker，返回
                    if let Some(syms) = &symbols {
                        if result.len() == syms.len() {
                            return Ok(result);
                        }
                    } else {
                        return Ok(result);
                    }
                } else {
                    // 处理单个ticker
                    if let Ok(ticker) = parser::parse_ws_ticker(&message, None) {
                        let symbol = ticker.symbol.clone();
                        result.insert(symbol.clone(), ticker.clone());

                        // 缓存ticker
                        let mut tickers = self.tickers.lock().await;
                        tickers.insert(symbol, ticker);

                        // 如果已经收到了所有请求的ticker，返回
                        if let Some(syms) = &symbols {
                            if result.len() == syms.len() {
                                return Ok(result);
                            }
                        }
                    }
                }
            }
        }
    }

    /// 获取缓存的ticker
    /// 处理订单簿增量更新（内部方法）
    ///
    /// # Arguments
    /// * `symbol` - 交易对
    /// * `delta_message` - WebSocket增量消息
    /// * `is_futures` - 是否是期货市场
    ///
    /// # Returns
    /// 处理结果，如果需要重同步则返回特殊错误
    async fn handle_orderbook_delta(
        &self,
        symbol: &str,
        delta_message: &Value,
        is_futures: bool,
    ) -> Result<()> {
        use ccxt_core::types::orderbook::{OrderBookDelta, OrderBookEntry};
        use rust_decimal::Decimal;

        // 解析增量消息
        let first_update_id = delta_message["U"]
            .as_i64()
            .ok_or_else(|| Error::invalid_request("Missing first update ID in delta message"))?;

        let final_update_id = delta_message["u"]
            .as_i64()
            .ok_or_else(|| Error::invalid_request("Missing final update ID in delta message"))?;

        let prev_final_update_id = if is_futures {
            delta_message["pu"].as_i64()
        } else {
            None
        };

        let timestamp = delta_message["E"]
            .as_i64()
            .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

        // 解析bids
        let mut bids = Vec::new();
        if let Some(bids_arr) = delta_message["b"].as_array() {
            for bid in bids_arr {
                if let (Some(price_str), Some(amount_str)) = (bid[0].as_str(), bid[1].as_str()) {
                    if let (Ok(price), Ok(amount)) =
                        (price_str.parse::<Decimal>(), amount_str.parse::<Decimal>())
                    {
                        bids.push(OrderBookEntry::new(Price::new(price), Amount::new(amount)));
                    }
                }
            }
        }

        // 解析asks
        let mut asks = Vec::new();
        if let Some(asks_arr) = delta_message["a"].as_array() {
            for ask in asks_arr {
                if let (Some(price_str), Some(amount_str)) = (ask[0].as_str(), ask[1].as_str()) {
                    if let (Ok(price), Ok(amount)) =
                        (price_str.parse::<Decimal>(), amount_str.parse::<Decimal>())
                    {
                        asks.push(OrderBookEntry::new(Price::new(price), Amount::new(amount)));
                    }
                }
            }
        }

        // 创建delta对象
        let delta = OrderBookDelta {
            symbol: symbol.to_string(),
            first_update_id,
            final_update_id,
            prev_final_update_id,
            timestamp,
            bids,
            asks,
        };

        // 获取或创建订单簿
        let mut orderbooks = self.orderbooks.lock().await;
        let orderbook = orderbooks
            .entry(symbol.to_string())
            .or_insert_with(|| OrderBook::new(symbol.to_string(), timestamp));

        // 如果订单簿未同步，缓冲delta
        if !orderbook.is_synced {
            orderbook.buffer_delta(delta);
            return Ok(());
        }

        // 应用delta更新
        if let Err(e) = orderbook.apply_delta(&delta, is_futures) {
            // 检查是否需要重同步
            if orderbook.needs_resync {
                tracing::warn!("Orderbook {} needs resync due to: {}", symbol, e);
                // 缓冲当前delta，它可能在重同步后有用
                orderbook.buffer_delta(delta);
                // 返回特殊错误指示需要重同步
                return Err(Error::invalid_request(format!("RESYNC_NEEDED: {}", e)));
            }
            return Err(Error::invalid_request(e));
        }

        Ok(())
    }

    /// 获取订单簿快照并初始化（内部方法）
    ///
    /// # Arguments
    /// * `exchange` - Exchange引用用于REST API调用
    /// * `symbol` - 交易对
    /// * `limit` - 深度档位
    /// * `is_futures` - 是否是期货市场
    ///
    /// # Returns
    /// 初始化的订单簿
    async fn fetch_orderbook_snapshot(
        &self,
        exchange: &Binance,
        symbol: &str,
        limit: Option<i64>,
        is_futures: bool,
    ) -> Result<OrderBook> {
        // 通过REST API获取快照
        let mut params = std::collections::HashMap::new();
        if let Some(l) = limit {
            // json! macro with simple values is infallible
            #[allow(clippy::disallowed_methods)]
            let limit_value = serde_json::json!(l);
            params.insert("limit".to_string(), limit_value);
        }

        let mut snapshot = exchange.fetch_order_book(symbol, None).await?;

        // 标记为已同步
        snapshot.is_synced = true;

        // 处理缓冲的增量消息
        let mut orderbooks = self.orderbooks.lock().await;
        if let Some(cached_ob) = orderbooks.get_mut(symbol) {
            // 将缓冲的delta移到快照中
            snapshot.buffered_deltas = cached_ob.buffered_deltas.clone();

            // 处理缓冲的delta
            if let Ok(processed) = snapshot.process_buffered_deltas(is_futures) {
                tracing::debug!("Processed {} buffered deltas for {}", processed, symbol);
            }
        }

        // 更新缓存
        orderbooks.insert(symbol.to_string(), snapshot.clone());

        Ok(snapshot)
    }

    /// Watch单个订单簿（内部方法）
    ///
    /// # Arguments
    /// * `exchange` - Exchange引用
    /// * `symbol` - 交易对（小写格式）
    /// * `limit` - 深度档位
    /// * `update_speed` - 更新速度（100或1000ms）
    /// * `is_futures` - 是否是期货市场
    ///
    /// # Returns
    /// 订单簿数据
    async fn watch_orderbook_internal(
        &self,
        exchange: &Binance,
        symbol: &str,
        limit: Option<i64>,
        update_speed: i32,
        is_futures: bool,
    ) -> Result<OrderBook> {
        // 构建stream名称
        let stream = if update_speed == 100 {
            format!("{}@depth@100ms", symbol.to_lowercase())
        } else {
            format!("{}@depth", symbol.to_lowercase())
        };

        // 订阅流
        self.client
            .subscribe(stream.clone(), Some(symbol.to_string()), None)
            .await?;

        // 开始接收增量消息并缓冲
        let snapshot_fetched = Arc::new(tokio::sync::Mutex::new(false));
        let _snapshot_fetched_clone = snapshot_fetched.clone();

        // 启动消息处理循环
        let _orderbooks_clone = self.orderbooks.clone();
        let _symbol_clone = symbol.to_string();

        tokio::spawn(async move {
            // 这里只是示意,实际需要从client接收消息
            // 实际实现会在主循环中处理
        });

        // 等待一些增量消息到达后获取快照
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // 初始快照获取
        let _snapshot = self
            .fetch_orderbook_snapshot(exchange, symbol, limit, is_futures)
            .await?;

        *snapshot_fetched.lock().await = true;

        // 主循环处理更新
        loop {
            if let Some(message) = self.client.receive().await {
                // 跳过订阅确认消息
                if message.get("result").is_some() {
                    continue;
                }

                // 检查是否是depth更新
                if let Some(event_type) = message.get("e").and_then(|v| v.as_str()) {
                    if event_type == "depthUpdate" {
                        // 处理增量更新
                        match self
                            .handle_orderbook_delta(symbol, &message, is_futures)
                            .await
                        {
                            Ok(_) => {
                                // 成功处理，返回更新后的订单簿
                                let orderbooks = self.orderbooks.lock().await;
                                if let Some(ob) = orderbooks.get(symbol) {
                                    if ob.is_synced {
                                        return Ok(ob.clone());
                                    }
                                }
                            }
                            Err(e) => {
                                let err_msg = e.to_string();

                                // 检查是否需要重同步
                                if err_msg.contains("RESYNC_NEEDED") {
                                    tracing::warn!("Resync needed for {}: {}", symbol, err_msg);

                                    // 检查速率限制
                                    let current_time = chrono::Utc::now().timestamp_millis();
                                    let should_resync = {
                                        let orderbooks = self.orderbooks.lock().await;
                                        if let Some(ob) = orderbooks.get(symbol) {
                                            ob.should_resync(current_time)
                                        } else {
                                            true
                                        }
                                    };

                                    if should_resync {
                                        tracing::info!("Initiating resync for {}", symbol);

                                        // 重置订单簿状态
                                        {
                                            let mut orderbooks = self.orderbooks.lock().await;
                                            if let Some(ob) = orderbooks.get_mut(symbol) {
                                                ob.reset_for_resync();
                                                ob.mark_resync_initiated(current_time);
                                            }
                                        }

                                        // 等待一些新的增量消息
                                        tokio::time::sleep(tokio::time::Duration::from_millis(500))
                                            .await;

                                        // 重新获取快照
                                        match self
                                            .fetch_orderbook_snapshot(
                                                exchange, symbol, limit, is_futures,
                                            )
                                            .await
                                        {
                                            Ok(_) => {
                                                tracing::info!(
                                                    "Resync completed successfully for {}",
                                                    symbol
                                                );
                                                // 继续循环处理后续更新
                                                continue;
                                            }
                                            Err(resync_err) => {
                                                tracing::error!(
                                                    "Resync failed for {}: {}",
                                                    symbol,
                                                    resync_err
                                                );
                                                return Err(resync_err);
                                            }
                                        }
                                    } else {
                                        tracing::debug!(
                                            "Resync rate limited for {}, skipping",
                                            symbol
                                        );
                                        continue;
                                    }
                                } else {
                                    tracing::error!(
                                        "Failed to handle orderbook delta: {}",
                                        err_msg
                                    );
                                    continue;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Watch多个订单簿（内部方法）
    ///
    /// # Arguments
    /// * `exchange` - Exchange引用
    /// * `symbols` - 交易对列表
    /// * `limit` - 深度档位
    /// * `update_speed` - 更新速度
    /// * `is_futures` - 是否是期货市场
    ///
    /// # Returns
    /// 订单簿数据映射
    async fn watch_orderbooks_internal(
        &self,
        exchange: &Binance,
        symbols: Vec<String>,
        limit: Option<i64>,
        update_speed: i32,
        is_futures: bool,
    ) -> Result<HashMap<String, OrderBook>> {
        // Binance限制最多200个交易对
        if symbols.len() > 200 {
            return Err(Error::invalid_request(
                "Binance supports max 200 symbols per connection",
            ));
        }

        // 批量订阅
        for symbol in &symbols {
            let stream = if update_speed == 100 {
                format!("{}@depth@100ms", symbol.to_lowercase())
            } else {
                format!("{}@depth", symbol.to_lowercase())
            };

            self.client
                .subscribe(stream, Some(symbol.clone()), None)
                .await?;
        }

        // 等待一些消息到达
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // 获取所有快照
        for symbol in &symbols {
            let _ = self
                .fetch_orderbook_snapshot(exchange, symbol, limit, is_futures)
                .await;
        }

        // 开始处理增量更新
        let mut result = HashMap::new();
        let mut update_count = 0;

        while update_count < symbols.len() {
            if let Some(message) = self.client.receive().await {
                // 跳过订阅确认
                if message.get("result").is_some() {
                    continue;
                }

                // 处理depth更新
                if let Some(event_type) = message.get("e").and_then(|v| v.as_str()) {
                    if event_type == "depthUpdate" {
                        if let Some(msg_symbol) = message.get("s").and_then(|v| v.as_str()) {
                            // 处理增量
                            if let Err(e) = self
                                .handle_orderbook_delta(msg_symbol, &message, is_futures)
                                .await
                            {
                                tracing::error!("Failed to handle orderbook delta: {}", e);
                                continue;
                            }

                            update_count += 1;
                        }
                    }
                }
            }
        }

        // 收集结果
        let orderbooks = self.orderbooks.lock().await;
        for symbol in &symbols {
            if let Some(ob) = orderbooks.get(symbol) {
                result.insert(symbol.clone(), ob.clone());
            }
        }

        Ok(result)
    }

    ///
    /// # Arguments
    /// * `symbol` - 交易对
    ///
    /// # Returns
    /// Ticker数据（如果存在）
    pub async fn get_cached_ticker(&self, symbol: &str) -> Option<Ticker> {
        let tickers = self.tickers.lock().await;
        tickers.get(symbol).cloned()
    }

    /// 获取所有缓存的tickers
    pub async fn get_all_cached_tickers(&self) -> HashMap<String, Ticker> {
        let tickers = self.tickers.lock().await;
        tickers.clone()
    }

    /// 处理余额更新消息（内部方法）
    ///
    /// # Arguments
    /// * `message` - WebSocket消息
    /// * `account_type` - 账户类型（spot/future/delivery等）
    ///
    /// # Returns
    /// 处理结果
    async fn handle_balance_message(&self, message: &Value, account_type: &str) -> Result<()> {
        use rust_decimal::Decimal;
        use std::str::FromStr;

        // 获取事件类型
        let event_type = message
            .get("e")
            .and_then(|e| e.as_str())
            .ok_or_else(|| Error::invalid_request("Missing event type in balance message"))?;

        // 获取或创建账户余额缓存
        let mut balances = self.balances.write().await;
        let balance = balances
            .entry(account_type.to_string())
            .or_insert_with(Balance::new);

        match event_type {
            // 单个资产增量更新（Spot）
            "balanceUpdate" => {
                let asset = message
                    .get("a")
                    .and_then(|a| a.as_str())
                    .ok_or_else(|| Error::invalid_request("Missing asset in balanceUpdate"))?;

                let delta_str = message
                    .get("d")
                    .and_then(|d| d.as_str())
                    .ok_or_else(|| Error::invalid_request("Missing delta in balanceUpdate"))?;

                let delta = Decimal::from_str(delta_str)
                    .map_err(|e| Error::invalid_request(format!("Invalid delta value: {}", e)))?;

                // 应用增量更新
                balance.apply_delta(asset.to_string(), delta);
            }

            // Spot账户完整余额更新
            "outboundAccountPosition" => {
                if let Some(balances_array) = message.get("B").and_then(|b| b.as_array()) {
                    for balance_item in balances_array {
                        let asset =
                            balance_item
                                .get("a")
                                .and_then(|a| a.as_str())
                                .ok_or_else(|| {
                                    Error::invalid_request("Missing asset in balance item")
                                })?;

                        let free_str = balance_item
                            .get("f")
                            .and_then(|f| f.as_str())
                            .ok_or_else(|| Error::invalid_request("Missing free balance"))?;

                        let locked_str = balance_item
                            .get("l")
                            .and_then(|l| l.as_str())
                            .ok_or_else(|| Error::invalid_request("Missing locked balance"))?;

                        let free = Decimal::from_str(free_str).map_err(|e| {
                            Error::invalid_request(format!("Invalid free value: {}", e))
                        })?;

                        let locked = Decimal::from_str(locked_str).map_err(|e| {
                            Error::invalid_request(format!("Invalid locked value: {}", e))
                        })?;

                        // 更新完整余额
                        balance.update_balance(asset.to_string(), free, locked);
                    }
                }
            }

            // 期货/交割账户更新
            "ACCOUNT_UPDATE" => {
                if let Some(account_data) = message.get("a") {
                    // 处理余额数组
                    if let Some(balances_array) = account_data.get("B").and_then(|b| b.as_array()) {
                        for balance_item in balances_array {
                            let asset = balance_item.get("a").and_then(|a| a.as_str()).ok_or_else(
                                || Error::invalid_request("Missing asset in balance item"),
                            )?;

                            let wallet_balance_str = balance_item
                                .get("wb")
                                .and_then(|wb| wb.as_str())
                                .ok_or_else(|| Error::invalid_request("Missing wallet balance"))?;

                            let wallet_balance =
                                Decimal::from_str(wallet_balance_str).map_err(|e| {
                                    Error::invalid_request(format!("Invalid wallet balance: {}", e))
                                })?;

                            // 可选的cross wallet balance
                            let cross_wallet = balance_item
                                .get("cw")
                                .and_then(|cw| cw.as_str())
                                .and_then(|s| Decimal::from_str(s).ok());

                            // 更新钱包余额
                            balance.update_wallet(asset.to_string(), wallet_balance, cross_wallet);
                        }
                    }

                    // 注意：持仓信息在 account_data.get("P") 中
                    // 这里暂不处理持仓，将在 watch_positions 中处理
                }
            }

            _ => {
                return Err(Error::invalid_request(format!(
                    "Unknown balance event type: {}",
                    event_type
                )));
            }
        }

        Ok(())
    }

    /// 解析WebSocket成交记录消息
    ///
    /// 从executionReport事件中提取成交信息
    ///
    /// # Arguments
    /// * `data` - WebSocket消息JSON数据
    ///
    /// # Returns
    /// 解析后的Trade对象
    fn parse_ws_trade(&self, data: &Value) -> Result<Trade> {
        use ccxt_core::types::{Fee, OrderSide, OrderType, TakerOrMaker};
        use rust_decimal::Decimal;
        use std::str::FromStr;

        // 提取基本字段
        let symbol = data
            .get("s")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::invalid_request("缺少symbol字段".to_string()))?
            .to_string();

        // 成交ID（字段t）
        let id = data
            .get("t")
            .and_then(|v| v.as_i64())
            .map(|v| v.to_string());

        // 成交时间（字段T）
        let timestamp = data.get("T").and_then(|v| v.as_i64()).unwrap_or(0);

        // 成交价格（字段L - Last executed price）
        let price = data
            .get("L")
            .and_then(|v| v.as_str())
            .and_then(|s| Decimal::from_str(s).ok())
            .unwrap_or(Decimal::ZERO);

        // 成交数量（字段l - Last executed quantity）
        let amount = data
            .get("l")
            .and_then(|v| v.as_str())
            .and_then(|s| Decimal::from_str(s).ok())
            .unwrap_or(Decimal::ZERO);

        // 成交金额（字段Y - Last quote asset transacted quantity，现货适用）
        let cost = data
            .get("Y")
            .and_then(|v| v.as_str())
            .and_then(|s| Decimal::from_str(s).ok())
            .or_else(|| {
                // 如果没有Y字段，通过价格*数量计算
                if price > Decimal::ZERO && amount > Decimal::ZERO {
                    Some(price * amount)
                } else {
                    None
                }
            });

        // 买卖方向（字段S）
        let side = data
            .get("S")
            .and_then(|v| v.as_str())
            .and_then(|s| match s.to_uppercase().as_str() {
                "BUY" => Some(OrderSide::Buy),
                "SELL" => Some(OrderSide::Sell),
                _ => None,
            })
            .unwrap_or(OrderSide::Buy);

        // 订单类型（字段o）
        let trade_type =
            data.get("o")
                .and_then(|v| v.as_str())
                .and_then(|s| match s.to_uppercase().as_str() {
                    "LIMIT" => Some(OrderType::Limit),
                    "MARKET" => Some(OrderType::Market),
                    _ => None,
                });

        // 订单ID（字段i）
        let order_id = data
            .get("i")
            .and_then(|v| v.as_i64())
            .map(|v| v.to_string());

        // Maker/Taker角色（字段m - Is the buyer the market maker?）
        let taker_or_maker = data.get("m").and_then(|v| v.as_bool()).map(|is_maker| {
            if is_maker {
                TakerOrMaker::Maker
            } else {
                TakerOrMaker::Taker
            }
        });

        // 手续费信息（字段n = fee amount, N = fee currency）
        let fee = if let Some(fee_cost_str) = data.get("n").and_then(|v| v.as_str()) {
            if let Ok(fee_cost) = Decimal::from_str(fee_cost_str) {
                let currency = data
                    .get("N")
                    .and_then(|v| v.as_str())
                    .unwrap_or("UNKNOWN")
                    .to_string();
                Some(Fee {
                    currency,
                    cost: fee_cost,
                    rate: None,
                })
            } else {
                None
            }
        } else {
            None
        };

        // 构建datetime字符串
        let datetime = chrono::DateTime::from_timestamp_millis(timestamp)
            .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string());

        // 构建info HashMap
        let mut info = HashMap::new();
        if let Value::Object(map) = data {
            for (k, v) in map.iter() {
                info.insert(k.clone(), v.clone());
            }
        }

        Ok(Trade {
            id,
            order: order_id,
            symbol,
            trade_type,
            side,
            taker_or_maker: taker_or_maker,
            price: Price::from(price),
            amount: Amount::from(amount),
            cost: cost.map(Cost::from),
            fee,
            timestamp,
            datetime,
            info,
        })
    }

    /// 过滤成交记录列表
    ///
    /// 根据symbol、时间范围和数量限制过滤成交记录
    async fn filter_my_trades(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<usize>,
    ) -> Result<Vec<Trade>> {
        let trades_map = self.my_trades.read().await;

        // 根据symbol过滤
        let mut trades: Vec<Trade> = if let Some(sym) = symbol {
            trades_map
                .get(sym)
                .map(|symbol_trades| symbol_trades.iter().cloned().collect())
                .unwrap_or_default()
        } else {
            trades_map
                .values()
                .flat_map(|symbol_trades| symbol_trades.iter().cloned())
                .collect()
        };

        // 根据since时间过滤
        if let Some(since_ts) = since {
            trades.retain(|trade| trade.timestamp >= since_ts);
        }

        // 按时间戳降序排序（最新的在前）
        trades.sort_by(|a, b| {
            let ts_a = a.timestamp;
            let ts_b = b.timestamp;
            ts_b.cmp(&ts_a)
        });

        // 应用limit限制
        if let Some(lim) = limit {
            trades.truncate(lim);
        }

        Ok(trades)
    }

    /// 解析WebSocket持仓数据
    ///
    /// # Arguments
    /// * `data` - WebSocket持仓数据（来自ACCOUNT_UPDATE事件的P数组元素）
    ///
    /// # Returns
    /// 解析后的Position对象
    ///
    /// # Binance WebSocket持仓数据格式
    /// ```json
    /// {
    ///   "s": "BTCUSDT",           // 交易对
    ///   "pa": "-0.089",           // 持仓数量（负数表示空头）
    ///   "ep": "19700.03933",      // 开仓价格
    ///   "cr": "-1260.24809979",   // 累计实现盈亏
    ///   "up": "1.53058860",       // 未实现盈亏
    ///   "mt": "isolated",         // 保证金模式：isolated/cross
    ///   "iw": "87.13658940",      // 逐仓钱包余额
    ///   "ps": "BOTH",             // 持仓方向：BOTH/LONG/SHORT
    ///   "ma": "USDT"              // 保证金资产
    /// }
    /// ```
    async fn parse_ws_position(&self, data: &serde_json::Value) -> Result<Position> {
        // 提取必需字段
        let symbol = data["s"]
            .as_str()
            .ok_or_else(|| Error::invalid_request("Missing symbol field"))?
            .to_string();

        let position_amount_str = data["pa"]
            .as_str()
            .ok_or_else(|| Error::invalid_request("Missing position amount"))?;

        let position_amount = position_amount_str
            .parse::<f64>()
            .map_err(|e| Error::invalid_request(format!("Invalid position amount: {}", e)))?;

        // 提取持仓方向字段
        let position_side = data["ps"]
            .as_str()
            .ok_or_else(|| Error::invalid_request("Missing position side"))?
            .to_uppercase();

        // 判断hedged模式和实际持仓方向
        // - 如果ps=BOTH，则hedged=false，根据pa的正负判断实际方向
        // - 如果ps=LONG/SHORT，则hedged=true，side就是ps的值
        let (side, hedged) = if position_side == "BOTH" {
            let actual_side = if position_amount < 0.0 {
                "short"
            } else {
                "long"
            };
            (actual_side.to_string(), false)
        } else {
            (position_side.to_lowercase(), true)
        };

        // 提取其他字段
        let entry_price = data["ep"].as_str().and_then(|s| s.parse::<f64>().ok());

        let unrealized_pnl = data["up"].as_str().and_then(|s| s.parse::<f64>().ok());

        let realized_pnl = data["cr"].as_str().and_then(|s| s.parse::<f64>().ok());

        let margin_mode = data["mt"].as_str().map(|s| s.to_string());

        let initial_margin = data["iw"].as_str().and_then(|s| s.parse::<f64>().ok());

        let _margin_asset = data["ma"].as_str().map(|s| s.to_string());

        // 构建Position对象
        Ok(Position {
            info: data.clone(),
            id: None,
            symbol,
            side: Some(side),
            contracts: Some(position_amount.abs()), // 合约数量取绝对值
            contract_size: None,
            entry_price,
            mark_price: None,
            notional: None,
            leverage: None,
            collateral: initial_margin, // 使用逐仓钱包余额作为抵押品
            initial_margin,
            initial_margin_percentage: None,
            maintenance_margin: None,
            maintenance_margin_percentage: None,
            unrealized_pnl,
            realized_pnl,
            liquidation_price: None,
            margin_ratio: None,
            margin_mode,
            hedged: Some(hedged),
            percentage: None,
            position_side: None,
            dual_side_position: None,
            timestamp: Some(chrono::Utc::now().timestamp_millis() as u64),
            datetime: Some(chrono::Utc::now().to_rfc3339()),
        })
    }

    /// 根据条件过滤持仓
    ///
    /// # Arguments
    /// * `symbols` - 交易对符号列表（可选）
    /// * `since` - 起始时间戳（可选）
    /// * `limit` - 返回数量限制（可选）
    ///
    /// # Returns
    /// 过滤后的持仓列表
    async fn filter_positions(
        &self,
        symbols: Option<&[String]>,
        since: Option<i64>,
        limit: Option<usize>,
    ) -> Result<Vec<Position>> {
        let positions_map = self.positions.read().await;

        // 根据symbols过滤
        let mut positions: Vec<Position> = if let Some(syms) = symbols {
            // 只返回指定symbols的持仓
            syms.iter()
                .filter_map(|sym| positions_map.get(sym))
                .flat_map(|side_map| side_map.values().cloned())
                .collect()
        } else {
            // 返回所有持仓
            positions_map
                .values()
                .flat_map(|side_map| side_map.values().cloned())
                .collect()
        };

        // 根据since时间过滤
        if let Some(since_ts) = since {
            positions.retain(|pos| {
                pos.timestamp
                    .map(|ts| ts as i64 >= since_ts)
                    .unwrap_or(false)
            });
        }

        // 按时间戳降序排序（最新的在前）
        positions.sort_by(|a, b| {
            let ts_a = a.timestamp.unwrap_or(0);
            let ts_b = b.timestamp.unwrap_or(0);
            ts_b.cmp(&ts_a)
        });

        // 应用limit限制
        if let Some(lim) = limit {
            positions.truncate(lim);
        }

        Ok(positions)
    }
}

impl Binance {
    /// 订阅ticker数据流
    ///
    /// # Arguments
    /// * `symbol` - 交易对
    ///
    /// # Returns
    /// 订阅结果
    pub async fn subscribe_ticker(&self, symbol: &str) -> Result<()> {
        let ws = self.create_ws();
        ws.connect().await?;

        // 转换交易对格式 BTC/USDT -> btcusdt
        let binance_symbol = symbol.replace('/', "").to_lowercase();
        ws.subscribe_ticker(&binance_symbol).await
    }

    /// 订阅交易记录数据流
    ///
    /// # Arguments
    /// * `symbol` - 交易对
    ///
    /// # Returns
    /// 订阅结果
    pub async fn subscribe_trades(&self, symbol: &str) -> Result<()> {
        let ws = self.create_ws();
        ws.connect().await?;

        let binance_symbol = symbol.replace('/', "").to_lowercase();
        ws.subscribe_trades(&binance_symbol).await
    }

    /// 订阅订单簿数据流
    ///
    /// # Arguments
    /// * `symbol` - 交易对
    /// * `levels` - 深度档位（默认20）
    ///
    /// # Returns
    /// 订阅结果
    pub async fn subscribe_orderbook(&self, symbol: &str, levels: Option<u32>) -> Result<()> {
        let ws = self.create_ws();
        ws.connect().await?;

        let binance_symbol = symbol.replace('/', "").to_lowercase();
        let depth_levels = levels.unwrap_or(20);
        ws.subscribe_orderbook(&binance_symbol, depth_levels, "1000ms")
            .await
    }

    /// 订阅K线数据流
    ///
    /// # Arguments
    /// * `symbol` - 交易对
    /// * `interval` - K线周期
    ///
    /// # Returns
    /// 订阅结果
    pub async fn subscribe_kline(&self, symbol: &str, interval: &str) -> Result<()> {
        let ws = self.create_ws();
        ws.connect().await?;

        let binance_symbol = symbol.replace('/', "").to_lowercase();
        ws.subscribe_kline(&binance_symbol, interval).await
    }

    /// Watch ticker - 观察单个交易对的ticker数据
    ///
    /// # Arguments
    /// * `symbol` - 统一交易对格式（如 "BTC/USDT"）
    /// * `params` - 可选参数
    ///   - `name`: 频道名称（ticker/miniTicker，默认ticker）
    ///
    /// # Returns
    /// Ticker数据
    ///
    /// # Example
    /// ```rust,no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # async fn example() -> ccxt_core::error::Result<()> {
    /// let exchange = Binance::new();
    /// let ticker = exchange.watch_ticker("BTC/USDT", None).await?;
    /// println!("Price: {}", ticker.last.unwrap_or(0.0));
    /// # Ok(())
    /// # }
    /// ```
    pub async fn watch_ticker(
        &self,
        symbol: &str,
        params: Option<HashMap<String, Value>>,
    ) -> Result<Ticker> {
        // 加载市场数据
        self.load_markets(false).await?;

        // 转换交易对格式
        let market = self.base.market(symbol).await?;
        let binance_symbol = market.id.to_lowercase();

        // 获取频道名称
        let channel_name = if let Some(p) = &params {
            p.get("name").and_then(|v| v.as_str()).unwrap_or("ticker")
        } else {
            "ticker"
        };

        // 创建WebSocket连接
        let ws = self.create_ws();
        ws.connect().await?;

        // Watch ticker
        ws.watch_ticker_internal(&binance_symbol, channel_name)
            .await
    }

    /// Watch tickers - 观察多个交易对的ticker数据
    ///
    /// # Arguments
    /// * `symbols` - 交易对列表（None表示所有交易对）
    /// * `params` - 可选参数
    ///   - `name`: 频道名称（ticker/miniTicker，默认ticker）
    ///
    /// # Returns
    /// Ticker数据映射（symbol -> Ticker）
    ///
    /// # Example
    /// ```rust,no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # async fn example() -> ccxt_core::error::Result<()> {
    /// let exchange = Binance::new();
    ///
    /// // Watch特定交易对
    /// let tickers = exchange.watch_tickers(
    ///     Some(vec!["BTC/USDT".to_string(), "ETH/USDT".to_string()]),
    ///     None
    /// ).await?;
    ///
    /// // Watch所有交易对
    /// let all_tickers = exchange.watch_tickers(None, None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn watch_tickers(
        &self,
        symbols: Option<Vec<String>>,
        params: Option<HashMap<String, Value>>,
    ) -> Result<HashMap<String, Ticker>> {
        // 加载市场数据
        self.load_markets(false).await?;

        // 获取频道名称
        let channel_name = if let Some(p) = &params {
            p.get("name").and_then(|v| v.as_str()).unwrap_or("ticker")
        } else {
            "ticker"
        };

        // 验证频道名称
        if channel_name == "bookTicker" {
            return Err(Error::invalid_request(
                "To subscribe for bids-asks, use watch_bids_asks() method instead",
            ));
        }

        // 转换交易对格式
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

        // 创建WebSocket连接
        let ws = self.create_ws();
        ws.connect().await?;

        // Watch tickers
        ws.watch_tickers_internal(binance_symbols, channel_name)
            .await
    }

    /// Watch mark price - 观察期货标记价格
    ///
    /// # Arguments
    /// * `symbol` - 统一交易对格式（如 "BTC/USDT:USDT"）
    /// * `params` - 可选参数
    ///   - `use1sFreq`: 是否使用1秒更新频率（默认true，否则3秒）
    ///
    /// # Returns
    /// MarkPrice数据（以Ticker格式返回）
    ///
    /// # Example
    /// ```rust,no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use std::collections::HashMap;
    /// # use serde_json::json;
    /// # async fn example() -> ccxt_core::error::Result<()> {
    /// let exchange = Binance::new();
    ///
    /// // 使用1秒更新
    /// let ticker = exchange.watch_mark_price("BTC/USDT:USDT", None).await?;
    ///
    /// // 使用3秒更新
    /// let mut params = HashMap::new();
    /// params.insert("use1sFreq".to_string(), json!(false));
    /// let ticker = exchange.watch_mark_price("BTC/USDT:USDT", Some(params)).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn watch_mark_price(
        &self,
        symbol: &str,
        params: Option<HashMap<String, Value>>,
    ) -> Result<Ticker> {
        // 加载市场数据
        self.load_markets(false).await?;

        // 验证是否是期货市场
        let market = self.base.market(symbol).await?;
        if market.market_type != MarketType::Swap && market.market_type != MarketType::Futures {
            return Err(Error::invalid_request(format!(
                "watch_mark_price() does not support {} markets",
                market.market_type
            )));
        }

        let binance_symbol = market.id.to_lowercase();

        // 获取更新频率
        let use_1s_freq = if let Some(p) = &params {
            p.get("use1sFreq").and_then(|v| v.as_bool()).unwrap_or(true)
        } else {
            true
        };

        // 构建频道名称
        let channel_name = if use_1s_freq {
            "markPrice@1s"
        } else {
            "markPrice"
        };

        // 创建WebSocket连接
        let ws = self.create_ws();
        ws.connect().await?;

        // Watch mark price
        ws.watch_ticker_internal(&binance_symbol, channel_name)
            .await
    }

    /// Watch order book - 观察订单簿深度数据
    ///
    /// # Arguments
    /// * `symbol` - 统一交易对格式（如 "BTC/USDT"）
    /// * `limit` - 深度档位（可选，默认无限制）
    /// * `params` - 可选参数
    ///   - `speed`: 更新速度（100或1000ms，默认100）
    ///
    /// # Returns
    /// 订单簿数据
    ///
    /// # Example
    /// ```rust,no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use std::collections::HashMap;
    /// # use serde_json::json;
    /// # async fn example() -> ccxt_core::error::Result<()> {
    /// let exchange = Binance::new();
    ///
    /// // Watch订单簿，100ms更新
    /// let orderbook = exchange.watch_order_book("BTC/USDT", None, None).await?;
    /// println!("Best bid: {:?}", orderbook.best_bid());
    /// println!("Best ask: {:?}", orderbook.best_ask());
    ///
    /// // Watch订单簿，限制100档，1000ms更新
    /// let mut params = HashMap::new();
    /// params.insert("speed".to_string(), json!(1000));
    /// let orderbook = exchange.watch_order_book(
    ///     "BTC/USDT",
    ///     Some(100),
    ///     Some(params)
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn watch_order_book(
        &self,
        symbol: &str,
        limit: Option<i64>,
        params: Option<HashMap<String, Value>>,
    ) -> Result<OrderBook> {
        // 加载市场数据
        self.load_markets(false).await?;

        // 获取市场信息
        let market = self.base.market(symbol).await?;
        let binance_symbol = market.id.to_lowercase();

        // 确定是否是期货市场
        let is_futures =
            market.market_type == MarketType::Swap || market.market_type == MarketType::Futures;

        // 获取更新速度
        let update_speed = if let Some(p) = &params {
            p.get("speed").and_then(|v| v.as_i64()).unwrap_or(100) as i32
        } else {
            100
        };

        // 验证更新速度
        if update_speed != 100 && update_speed != 1000 {
            return Err(Error::invalid_request(
                "Update speed must be 100 or 1000 milliseconds",
            ));
        }

        // 创建WebSocket连接
        let ws = self.create_ws();
        ws.connect().await?;

        // Watch订单簿
        ws.watch_orderbook_internal(self, &binance_symbol, limit, update_speed, is_futures)
            .await
    }

    /// Watch order books - 观察多个订单簿
    ///
    /// # Arguments
    /// * `symbols` - 交易对列表（最多200个）
    /// * `limit` - 深度档位
    /// * `params` - 可选参数
    ///   - `speed`: 更新速度（100或1000ms）
    ///
    /// # Returns
    /// 订单簿数据映射
    ///
    /// # Example
    /// ```rust,no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # async fn example() -> ccxt_core::error::Result<()> {
    /// let exchange = Binance::new();
    ///
    /// let symbols = vec![
    ///     "BTC/USDT".to_string(),
    ///     "ETH/USDT".to_string(),
    /// ];
    ///
    /// let orderbooks = exchange.watch_order_books(symbols, None, None).await?;
    /// for (symbol, ob) in orderbooks {
    ///     println!("{}: spread = {:?}", symbol, ob.spread());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn watch_order_books(
        &self,
        symbols: Vec<String>,
        limit: Option<i64>,
        params: Option<HashMap<String, Value>>,
    ) -> Result<HashMap<String, OrderBook>> {
        // 验证数量限制
        if symbols.is_empty() {
            return Err(Error::invalid_request("Symbols list cannot be empty"));
        }

        if symbols.len() > 200 {
            return Err(Error::invalid_request(
                "Binance supports max 200 symbols per connection",
            ));
        }

        // 加载市场数据
        self.load_markets(false).await?;

        // 转换交易对格式并检查市场类型
        let mut binance_symbols = Vec::new();
        let mut is_futures = false;

        for symbol in &symbols {
            let market = self.base.market(symbol).await?;
            binance_symbols.push(market.id.to_lowercase());

            // 检查市场类型一致性
            let current_is_futures =
                market.market_type == MarketType::Swap || market.market_type == MarketType::Futures;
            if !binance_symbols.is_empty() && current_is_futures != is_futures {
                return Err(Error::invalid_request(
                    "Cannot mix spot and futures markets in watch_order_books",
                ));
            }
            is_futures = current_is_futures;
        }

        // 获取更新速度
        let update_speed = if let Some(p) = &params {
            p.get("speed").and_then(|v| v.as_i64()).unwrap_or(100) as i32
        } else {
            100
        };

        // 创建WebSocket连接
        let ws = self.create_ws();
        ws.connect().await?;

        // Watch多个订单簿
        ws.watch_orderbooks_internal(self, binance_symbols, limit, update_speed, is_futures)
            .await
    }

    /// Watch mark prices - 观察多个期货标记价格
    ///
    /// # Arguments
    /// * `symbols` - 交易对列表（None表示所有交易对）
    /// * `params` - 可选参数
    ///   - `use1sFreq`: 是否使用1秒更新频率（默认true）
    ///
    /// # Returns
    /// Ticker数据映射
    pub async fn watch_mark_prices(
        &self,
        symbols: Option<Vec<String>>,
        params: Option<HashMap<String, Value>>,
    ) -> Result<HashMap<String, Ticker>> {
        // 加载市场数据
        self.load_markets(false).await?;

        // 获取更新频率
        let use_1s_freq = if let Some(p) = &params {
            p.get("use1sFreq").and_then(|v| v.as_bool()).unwrap_or(true)
        } else {
            true
        };

        // 构建频道名称
        let channel_name = if use_1s_freq {
            "markPrice@1s"
        } else {
            "markPrice"
        };

        // 转换交易对格式并验证
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

        // 创建WebSocket连接
        let ws = self.create_ws();
        ws.connect().await?;

        // Watch mark prices
        ws.watch_tickers_internal(binance_symbols, channel_name)
            .await
    }
    /// 监控交易数据流
    ///
    /// # Arguments
    ///
    /// * `symbol` - 交易对符号（统一格式，如 "BTC/USDT"）
    /// * `since` - 起始时间戳（毫秒），可选
    /// * `limit` - 返回数据数量限制，可选
    ///
    /// # Returns
    ///
    /// 返回交易数据数组
    pub async fn watch_trades(
        &self,
        symbol: &str,
        since: Option<i64>,
        limit: Option<usize>,
    ) -> Result<Vec<Trade>> {
        // 确保市场数据已加载
        self.base.load_markets(false).await?;

        // 获取市场信息
        let market = self.base.market(symbol).await?;
        let binance_symbol = market.id.to_lowercase();

        // 创建WebSocket连接
        let ws = self.create_ws();
        ws.connect().await?;

        // 订阅交易数据流
        ws.subscribe_trades(&binance_symbol).await?;

        // 等待并处理消息
        let mut retries = 0;
        const MAX_RETRIES: u32 = 50;

        while retries < MAX_RETRIES {
            if let Some(msg) = ws.client.receive().await {
                // 跳过订阅确认消息
                if msg.get("result").is_some() || msg.get("id").is_some() {
                    continue;
                }

                // 解析交易数据
                if let Ok(trade) = parser::parse_ws_trade(&msg, Some(&market)) {
                    // 缓存交易数据
                    let mut trades_map = ws.trades.lock().await;
                    let trades = trades_map
                        .entry(symbol.to_string())
                        .or_insert_with(VecDeque::new);

                    // 限制缓存大小
                    const MAX_TRADES: usize = 1000;
                    if trades.len() >= MAX_TRADES {
                        trades.pop_front();
                    }
                    trades.push_back(trade);

                    // 获取返回数据
                    let mut result: Vec<Trade> = trades.iter().cloned().collect();

                    // 应用since过滤
                    if let Some(since_ts) = since {
                        result.retain(|t| t.timestamp >= since_ts);
                    }

                    // 应用limit限制
                    if let Some(limit_size) = limit {
                        if result.len() > limit_size {
                            result = result.split_off(result.len() - limit_size);
                        }
                    }

                    return Ok(result);
                }
            }

            retries += 1;
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        Err(Error::network("Timeout waiting for trade data"))
    }

    /// 监控K线/OHLCV数据流
    ///
    /// # Arguments
    ///
    /// * `symbol` - 交易对符号（统一格式，如 "BTC/USDT"）
    /// * `timeframe` - K线时间周期（如 "1m", "5m", "1h", "1d"）
    /// * `since` - 起始时间戳（毫秒），可选
    /// * `limit` - 返回数据数量限制，可选
    ///
    /// # Returns
    ///
    /// 返回OHLCV数据数组
    pub async fn watch_ohlcv(
        &self,
        symbol: &str,
        timeframe: &str,
        since: Option<i64>,
        limit: Option<usize>,
    ) -> Result<Vec<OHLCV>> {
        // 确保市场数据已加载
        self.base.load_markets(false).await?;

        // 获取市场信息
        let market = self.base.market(symbol).await?;
        let binance_symbol = market.id.to_lowercase();

        // 创建WebSocket连接
        let ws = self.create_ws();
        ws.connect().await?;

        // 订阅K线数据流
        ws.subscribe_kline(&binance_symbol, timeframe).await?;

        // 等待并处理消息
        let mut retries = 0;
        const MAX_RETRIES: u32 = 50;

        while retries < MAX_RETRIES {
            if let Some(msg) = ws.client.receive().await {
                // 跳过订阅确认消息
                if msg.get("result").is_some() || msg.get("id").is_some() {
                    continue;
                }

                // 解析K线数据
                if let Ok(ohlcv) = parser::parse_ws_ohlcv(&msg) {
                    // 缓存K线数据
                    let cache_key = format!("{}:{}", symbol, timeframe);
                    let mut ohlcvs_map = ws.ohlcvs.lock().await;
                    let ohlcvs = ohlcvs_map.entry(cache_key).or_insert_with(VecDeque::new);

                    // 限制缓存大小
                    const MAX_OHLCVS: usize = 1000;
                    if ohlcvs.len() >= MAX_OHLCVS {
                        ohlcvs.pop_front();
                    }
                    ohlcvs.push_back(ohlcv);

                    // 获取返回数据
                    let mut result: Vec<OHLCV> = ohlcvs.iter().cloned().collect();

                    // 应用since过滤
                    if let Some(since_ts) = since {
                        result.retain(|o| o.timestamp >= since_ts);
                    }

                    // 应用limit限制
                    if let Some(limit_size) = limit {
                        if result.len() > limit_size {
                            result = result.split_off(result.len() - limit_size);
                        }
                    }

                    return Ok(result);
                }
            }

            retries += 1;
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        Err(Error::network("Timeout waiting for OHLCV data"))
    }

    /// 监控最佳买卖价数据流
    ///
    /// # Arguments
    ///
    /// * `symbol` - 交易对符号（统一格式，如 "BTC/USDT"）
    ///
    /// # Returns
    ///
    /// 返回最佳买卖价数据
    pub async fn watch_bids_asks(&self, symbol: &str) -> Result<BidAsk> {
        // 确保市场数据已加载
        self.base.load_markets(false).await?;

        // 获取市场信息
        let market = self.base.market(symbol).await?;
        let binance_symbol = market.id.to_lowercase();

        // 创建WebSocket连接
        let ws = self.create_ws();
        ws.connect().await?;

        // 订阅bookTicker数据流
        let stream_name = format!("{}@bookTicker", binance_symbol);
        ws.client
            .subscribe(stream_name, Some(symbol.to_string()), None)
            .await?;

        // 等待并处理消息
        let mut retries = 0;
        const MAX_RETRIES: u32 = 50;

        while retries < MAX_RETRIES {
            if let Some(msg) = ws.client.receive().await {
                // 跳过订阅确认消息
                if msg.get("result").is_some() || msg.get("id").is_some() {
                    continue;
                }

                // 解析BidAsk数据
                if let Ok(bid_ask) = parser::parse_ws_bid_ask(&msg) {
                    // 缓存数据
                    let mut bids_asks_map = ws.bids_asks.lock().await;
                    bids_asks_map.insert(symbol.to_string(), bid_ask.clone());

                    return Ok(bid_ask);
                }
            }

            retries += 1;
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        Err(Error::network("Timeout waiting for BidAsk data"))
    }

    /// 监控账户余额变化（私有数据流）
    ///
    /// # Arguments
    ///
    /// * `params` - 可选参数
    ///   - `type`: 账户类型（spot/future/delivery/margin等）
    ///   - `fetchBalanceSnapshot`: 是否获取初始快照（默认false）
    ///   - `awaitBalanceSnapshot`: 是否等待快照完成（默认true）
    ///
    /// # Returns
    ///
    /// 返回账户余额数据
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # use std::collections::HashMap;
    /// # use serde_json::json;
    /// # async fn example() -> ccxt_core::error::Result<()> {
    /// let mut config = ExchangeConfig::default();
    /// config.api_key = Some("your-api-key".to_string());
    /// config.secret = Some("your-secret".to_string());
    /// let exchange = Binance::new(config)?;
    ///
    /// // Watch现货账户余额
    /// let balance = exchange.watch_balance(None).await?;
    ///
    /// // Watch期货账户余额
    /// let mut params = HashMap::new();
    /// 创建带认证的WebSocket连接
    /// params.insert("type".to_string(), json!("future"));
    /// let futures_balance = exchange.watch_balance(Some(params)).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn watch_balance(
        self: Arc<Self>,
        params: Option<HashMap<String, Value>>,
    ) -> Result<Balance> {
        // 确保市场数据已加载
        self.base.load_markets(false).await?;

        // 获取账户类型
        let account_type = if let Some(p) = &params {
            p.get("type")
                .and_then(|v| v.as_str())
                .unwrap_or(&self.options.default_type)
        } else {
            &self.options.default_type
        };

        // 获取配置选项
        let fetch_snapshot = if let Some(p) = &params {
            p.get("fetchBalanceSnapshot")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        } else {
            false
        };

        let await_snapshot = if let Some(p) = &params {
            p.get("awaitBalanceSnapshot")
                .and_then(|v| v.as_bool())
                .unwrap_or(true)
        } else {
            true
        };

        // 创建带认证的WebSocket连接
        let ws = self.create_authenticated_ws();
        ws.connect().await?;

        // 如果需要获取初始快照
        if fetch_snapshot {
            let snapshot = self.fetch_balance(params.clone()).await?;

            // 更新缓存
            let mut balances = ws.balances.write().await;
            balances.insert(account_type.to_string(), snapshot.clone());

            if !await_snapshot {
                return Ok(snapshot);
            }
        }

        // 等待并处理余额更新消息
        let mut retries = 0;
        const MAX_RETRIES: u32 = 100;

        while retries < MAX_RETRIES {
            if let Some(msg) = ws.client.receive().await {
                // 跳过订阅确认消息
                if msg.get("result").is_some() || msg.get("id").is_some() {
                    continue;
                }

                // 获取事件类型
                if let Some(event_type) = msg.get("e").and_then(|e| e.as_str()) {
                    // 处理不同类型的余额消息
                    match event_type {
                        "balanceUpdate" | "outboundAccountPosition" | "ACCOUNT_UPDATE" => {
                            // 解析并更新余额
                            if let Ok(()) = ws.handle_balance_message(&msg, account_type).await {
                                // 返回更新后的余额
                                let balances = ws.balances.read().await;
                                if let Some(balance) = balances.get(account_type) {
                                    return Ok(balance.clone());
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }

            retries += 1;
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        Err(Error::network("Timeout waiting for balance data"))
    }

    /// 监听订单更新（私有WebSocket）
    ///
    /// 通过用户数据流接收订单状态变化的实时更新
    ///
    /// # 参数
    /// * `symbol` - 可选的交易对过滤（例如"BTC/USDT"）
    /// * `since` - 可选的起始时间戳（毫秒）
    /// * `limit` - 可选的返回数量限制
    /// * `params` - 可选的额外参数
    ///
    /// # 返回
    /// 返回订单列表，按时间降序排列
    ///
    /// # 示例
    /// ```no_run
    /// use std::sync::Arc;
    /// use ccxt_exchanges::binance::Binance;
    ///
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    ///     let exchange = Arc::new(Binance::new(Default::default()));
    ///
    ///     // 监听所有交易对的订单更新
    ///     let orders = exchange.watch_orders(None, None, None, None).await?;
    ///     println!("收到 {} 个订单更新", orders.len());
    ///
    ///     // 监听特定交易对的订单更新
    ///     let btc_orders = exchange.watch_orders(Some("BTC/USDT"), None, None, None).await?;
    ///     println!("BTC/USDT 订单: {:?}", btc_orders);
    ///
    ///     Ok(())
    /// }
    /// ```
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

        // 循环接收消息
        loop {
            if let Some(msg) = ws.client.receive().await {
                if let Value::Object(data) = msg {
                    if let Some(event_type) = data.get("e").and_then(|v| v.as_str()) {
                        match event_type {
                            "executionReport" => {
                                // 解析订单数据
                                let order = self.parse_ws_order(&data)?;

                                // 更新订单缓存
                                let mut orders = ws.orders.write().await;
                                let symbol_orders = orders
                                    .entry(order.symbol.clone())
                                    .or_insert_with(HashMap::new);
                                symbol_orders.insert(order.id.clone(), order.clone());
                                drop(orders);

                                // 检查是否为成交事件（executionType = "TRADE"）
                                if let Some(exec_type) = data.get("x").and_then(|v| v.as_str()) {
                                    if exec_type == "TRADE" {
                                        // 解析成交记录
                                        if let Ok(trade) =
                                            ws.parse_ws_trade(&Value::Object(data.clone()))
                                        {
                                            // 更新成交记录缓存
                                            let mut trades = ws.my_trades.write().await;
                                            let symbol_trades = trades
                                                .entry(trade.symbol.clone())
                                                .or_insert_with(VecDeque::new);

                                            // 添加新成交记录（保持最多1000条）
                                            symbol_trades.push_front(trade);
                                            if symbol_trades.len() > 1000 {
                                                symbol_trades.pop_back();
                                            }
                                        }
                                    }
                                }

                                // 返回过滤后的订单列表
                                return self.filter_orders(&ws, symbol, since, limit).await;
                            }
                            _ => continue,
                        }
                    }
                }
            } else {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
    }

    /// 解析WebSocket订单消息
    fn parse_ws_order(&self, data: &serde_json::Map<String, Value>) -> Result<Order> {
        use ccxt_core::types::{OrderSide, OrderStatus, OrderType};
        use rust_decimal::Decimal;
        use std::str::FromStr;

        // 提取基本字段
        let symbol = data.get("s").and_then(|v| v.as_str()).unwrap_or("");
        let order_id = data
            .get("i")
            .and_then(|v| v.as_i64())
            .map(|id| id.to_string())
            .unwrap_or_default();
        let client_order_id = data.get("c").and_then(|v| v.as_str()).map(String::from);

        // 解析订单状态
        let status_str = data.get("X").and_then(|v| v.as_str()).unwrap_or("NEW");
        let status = match status_str {
            "NEW" => OrderStatus::Open,
            "PARTIALLY_FILLED" => OrderStatus::Open,
            "FILLED" => OrderStatus::Closed,
            "CANCELED" => OrderStatus::Canceled,
            "REJECTED" => OrderStatus::Rejected,
            "EXPIRED" => OrderStatus::Expired,
            _ => OrderStatus::Open,
        };

        // 解析订单方向
        let side_str = data.get("S").and_then(|v| v.as_str()).unwrap_or("BUY");
        let side = match side_str {
            "BUY" => OrderSide::Buy,
            "SELL" => OrderSide::Sell,
            _ => OrderSide::Buy,
        };

        // 解析订单类型
        let type_str = data.get("o").and_then(|v| v.as_str()).unwrap_or("LIMIT");
        let order_type = match type_str {
            "LIMIT" => OrderType::Limit,
            "MARKET" => OrderType::Market,
            "STOP_LOSS" => OrderType::StopLoss,
            "STOP_LOSS_LIMIT" => OrderType::StopLossLimit,
            "TAKE_PROFIT" => OrderType::TakeProfit,
            "TAKE_PROFIT_LIMIT" => OrderType::TakeProfitLimit,
            "LIMIT_MAKER" => OrderType::LimitMaker,
            _ => OrderType::Limit,
        };

        // 解析数量和价格
        let amount = data
            .get("q")
            .and_then(|v| v.as_str())
            .and_then(|s| Decimal::from_str(s).ok())
            .unwrap_or(Decimal::ZERO);

        let price = data
            .get("p")
            .and_then(|v| v.as_str())
            .and_then(|s| Decimal::from_str(s).ok());

        let filled = data
            .get("z")
            .and_then(|v| v.as_str())
            .and_then(|s| Decimal::from_str(s).ok());

        let cost = data
            .get("Z")
            .and_then(|v| v.as_str())
            .and_then(|s| Decimal::from_str(s).ok());

        // 计算剩余数量
        let remaining = match filled {
            Some(fill) => Some(amount - fill),
            None => None,
        };

        // 计算平均价格
        let average = match (filled, cost) {
            (Some(fill), Some(c)) if fill > Decimal::ZERO && c > Decimal::ZERO => Some(c / fill),
            _ => None,
        };

        // 解析时间戳
        let timestamp = data.get("T").and_then(|v| v.as_i64());
        let last_trade_timestamp = data.get("T").and_then(|v| v.as_i64());

        Ok(Order {
            id: order_id,
            client_order_id,
            timestamp,
            datetime: timestamp.map(|ts| {
                chrono::DateTime::from_timestamp_millis(ts)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default()
            }),
            last_trade_timestamp,
            symbol: symbol.to_string(),
            order_type,
            side,
            price,
            average,
            amount,
            cost,
            filled,
            remaining,
            status,
            fee: None,
            fees: None,
            trades: None,
            time_in_force: data.get("f").and_then(|v| v.as_str()).map(String::from),
            post_only: None,
            reduce_only: None,
            stop_price: data
                .get("P")
                .and_then(|v| v.as_str())
                .and_then(|s| Decimal::from_str(s).ok()),
            trigger_price: None,
            take_profit_price: None,
            stop_loss_price: None,
            trailing_delta: None,
            trailing_percent: None,
            activation_price: None,
            callback_rate: None,
            working_type: data.get("wt").and_then(|v| v.as_str()).map(String::from),
            info: data.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        })
    }

    /// 过滤订单列表
    async fn filter_orders(
        &self,
        ws: &BinanceWs,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<usize>,
    ) -> Result<Vec<Order>> {
        let orders_map = ws.orders.read().await;

        // 根据symbol过滤
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

        // 根据since时间过滤
        if let Some(since_ts) = since {
            orders.retain(|order| order.timestamp.map_or(false, |ts| ts >= since_ts));
        }

        // 按时间戳降序排序
        orders.sort_by(|a, b| {
            let ts_a = a.timestamp.unwrap_or(0);
            let ts_b = b.timestamp.unwrap_or(0);
            ts_b.cmp(&ts_a)
        });

        // 应用limit限制
        if let Some(lim) = limit {
            orders.truncate(lim);
        }

        Ok(orders)
    }

    /// 订阅用户成交记录更新（需要认证）
    ///
    /// # Arguments
    /// * `symbol` - 交易对符号（可选，如None则订阅所有交易对）
    /// * `since` - 起始时间戳（毫秒）
    /// * `limit` - 返回数量限制
    /// * `params` - 额外参数
    ///
    /// # Returns
    /// 成交记录列表
    ///
    /// # Example
    /// ```rust,no_run
    /// use std::sync::Arc;
    /// use ccxt_exchanges::binance::Binance;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let exchange = Arc::new(Binance::new(Default::default()));
    ///     let ws = exchange.create_authenticated_ws();
    ///     
    ///     // 订阅BTC/USDT的成交记录
    ///     let trades = ws.watch_my_trades(Some("BTC/USDT"), None, None, None).await.unwrap();
    ///     println!("My trades: {:?}", trades);
    /// }
    /// ```
    pub async fn watch_my_trades(
        self: Arc<Self>,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<usize>,
        _params: Option<HashMap<String, Value>>,
    ) -> Result<Vec<Trade>> {
        // 创建带认证的WebSocket连接
        let ws = self.create_authenticated_ws();
        ws.connect().await?;

        // 等待并处理成交记录消息
        let mut retries = 0;
        const MAX_RETRIES: u32 = 100;

        while retries < MAX_RETRIES {
            if let Some(msg) = ws.client.receive().await {
                // 跳过订阅确认消息
                if msg.get("result").is_some() || msg.get("id").is_some() {
                    continue;
                }

                // 获取事件类型
                if let Some(event_type) = msg.get("e").and_then(|e| e.as_str()) {
                    // 处理executionReport事件（包含成交信息）
                    if event_type == "executionReport" {
                        // 解析成交记录
                        if let Ok(trade) = ws.parse_ws_trade(&msg) {
                            let symbol_key = trade.symbol.clone();

                            // 更新缓存
                            let mut trades_map = ws.my_trades.write().await;
                            let symbol_trades =
                                trades_map.entry(symbol_key).or_insert_with(VecDeque::new);

                            // 添加到队列头部（最新的在前）
                            symbol_trades.push_front(trade);

                            // 限制每个symbol最多保留1000条记录
                            if symbol_trades.len() > 1000 {
                                symbol_trades.pop_back();
                            }
                        }
                    }
                }
            } else {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }

            retries += 1;
        }

        // 过滤并返回成交记录
        ws.filter_my_trades(symbol, since, limit).await
    }

    /// Watch Positions - 观察期货持仓变化（需要认证）
    ///
    /// 监听用户的期货持仓变化，通过WebSocket实时接收ACCOUNT_UPDATE事件。
    /// 支持USDⓈ-M合约和COIN-M合约。
    ///
    /// # Arguments
    /// * `symbols` - 交易对符号列表（可选，None表示订阅所有持仓）
    /// * `since` - 起始时间戳（可选）
    /// * `limit` - 返回数量限制（可选）
    /// * `params` - 可选参数
    ///   - `type`: 市场类型（future/delivery，默认future）
    ///   - `subType`: 子类型（linear/inverse）
    ///
    /// # Returns
    /// 持仓列表
    ///
    /// # 实现细节
    /// 1. 通过User Data Stream接收ACCOUNT_UPDATE事件
    /// 2. 解析事件中的持仓数据（P数组）
    /// 3. 更新内部持仓缓存
    /// 4. 根据过滤条件返回结果
    ///
    /// # WebSocket消息格式
    /// ```json
    /// {
    ///   "e": "ACCOUNT_UPDATE",
    ///   "T": 1667881353112,
    ///   "E": 1667881353115,
    ///   "a": {
    ///     "P": [
    ///       {
    ///         "s": "BTCUSDT",
    ///         "pa": "-0.089",
    ///         "ep": "19700.03933",
    ///         "up": "1.53058860",
    ///         "mt": "isolated",
    ///         "ps": "BOTH"
    ///       }
    ///     ]
    ///   }
    /// }
    /// ```
    ///
    /// # Example
    /// ```rust,no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use std::sync::Arc;
    /// # async fn example() -> ccxt_core::error::Result<()> {
    /// let exchange = Arc::new(Binance::new());
    ///
    /// // 观察所有持仓
    /// let positions = exchange.watch_positions(None, None, None, None).await?;
    /// for pos in positions {
    ///     println!("Symbol: {}, Side: {:?}, Contracts: {:?}",
    ///              pos.symbol, pos.side, pos.contracts);
    /// }
    ///
    /// // 观察特定交易对的持仓
    /// let symbols = vec!["BTC/USDT".to_string(), "ETH/USDT".to_string()];
    /// let positions = exchange.watch_positions(Some(symbols), None, Some(10), None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn watch_positions(
        self: Arc<Self>,
        symbols: Option<Vec<String>>,
        since: Option<i64>,
        limit: Option<usize>,
        _params: Option<HashMap<String, Value>>,
    ) -> Result<Vec<Position>> {
        // 创建带认证的WebSocket连接
        let ws = self.create_authenticated_ws();
        ws.connect().await?;

        // 等待并处理持仓更新消息
        let mut retries = 0;
        const MAX_RETRIES: u32 = 100;

        while retries < MAX_RETRIES {
            if let Some(msg) = ws.client.receive().await {
                // 跳过订阅确认消息
                if msg.get("result").is_some() || msg.get("id").is_some() {
                    continue;
                }

                // 获取事件类型
                if let Some(event_type) = msg.get("e").and_then(|e| e.as_str()) {
                    // 处理ACCOUNT_UPDATE事件
                    if event_type == "ACCOUNT_UPDATE" {
                        // 提取持仓数据
                        if let Some(account_data) = msg.get("a") {
                            if let Some(positions_array) =
                                account_data.get("P").and_then(|p| p.as_array())
                            {
                                // 解析并更新每个持仓
                                for position_data in positions_array {
                                    if let Ok(position) = ws.parse_ws_position(position_data).await
                                    {
                                        let symbol_key = position.symbol.clone();
                                        let side_key = position
                                            .side
                                            .clone()
                                            .unwrap_or_else(|| "both".to_string());

                                        // 更新缓存
                                        let mut positions_map = ws.positions.write().await;
                                        let symbol_positions = positions_map
                                            .entry(symbol_key)
                                            .or_insert_with(HashMap::new);

                                        // 如果持仓数量为0，从缓存中移除
                                        if position.contracts.unwrap_or(0.0).abs() < 0.000001 {
                                            symbol_positions.remove(&side_key);
                                            // 如果该symbol没有任何持仓了，移除整个symbol
                                            if symbol_positions.is_empty() {
                                                positions_map.remove(&position.symbol);
                                            }
                                        } else {
                                            // 更新或添加持仓
                                            symbol_positions.insert(side_key, position);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }

            retries += 1;
        }

        // 过滤并返回持仓
        let symbols_ref = symbols.as_ref().map(|v| v.as_slice());
        ws.filter_positions(symbols_ref, since, limit).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binance_ws_creation() {
        let ws = BinanceWs::new(WS_BASE_URL.to_string());
        // 基本创建测试
        assert!(!ws.listen_key.try_read().is_err());
    }

    #[test]
    fn test_stream_format() {
        let symbol = "btcusdt";

        // Ticker流格式
        let ticker_stream = format!("{}@ticker", symbol);
        assert_eq!(ticker_stream, "btcusdt@ticker");

        // 交易流格式
        let trade_stream = format!("{}@trade", symbol);
        assert_eq!(trade_stream, "btcusdt@trade");

        // 深度流格式
        let depth_stream = format!("{}@depth20", symbol);
        assert_eq!(depth_stream, "btcusdt@depth20");

        // K线流格式
        let kline_stream = format!("{}@kline_1m", symbol);
        assert_eq!(kline_stream, "btcusdt@kline_1m");
    }

    #[tokio::test]
    async fn test_subscription_manager_basic() {
        let manager = SubscriptionManager::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();

        // 测试初始状态
        assert_eq!(manager.active_count(), 0);
        assert!(!manager.has_subscription("btcusdt@ticker").await);

        // 测试添加订阅
        manager
            .add_subscription(
                "btcusdt@ticker".to_string(),
                "BTCUSDT".to_string(),
                SubscriptionType::Ticker,
                tx.clone(),
            )
            .await
            .unwrap();

        assert_eq!(manager.active_count(), 1);
        assert!(manager.has_subscription("btcusdt@ticker").await);

        // 测试获取订阅
        let sub = manager.get_subscription("btcusdt@ticker").await;
        assert!(sub.is_some());
        let sub = sub.unwrap();
        assert_eq!(sub.stream, "btcusdt@ticker");
        assert_eq!(sub.symbol, "BTCUSDT");
        assert_eq!(sub.sub_type, SubscriptionType::Ticker);

        // 测试移除订阅
        manager.remove_subscription("btcusdt@ticker").await.unwrap();
        assert_eq!(manager.active_count(), 0);
        assert!(!manager.has_subscription("btcusdt@ticker").await);
    }

    #[tokio::test]
    async fn test_subscription_manager_multiple() {
        let manager = SubscriptionManager::new();
        let (tx1, _rx1) = tokio::sync::mpsc::unbounded_channel();
        let (tx2, _rx2) = tokio::sync::mpsc::unbounded_channel();
        let (tx3, _rx3) = tokio::sync::mpsc::unbounded_channel();

        // 添加多个订阅
        manager
            .add_subscription(
                "btcusdt@ticker".to_string(),
                "BTCUSDT".to_string(),
                SubscriptionType::Ticker,
                tx1,
            )
            .await
            .unwrap();

        manager
            .add_subscription(
                "btcusdt@depth".to_string(),
                "BTCUSDT".to_string(),
                SubscriptionType::OrderBook,
                tx2,
            )
            .await
            .unwrap();

        manager
            .add_subscription(
                "ethusdt@ticker".to_string(),
                "ETHUSDT".to_string(),
                SubscriptionType::Ticker,
                tx3,
            )
            .await
            .unwrap();

        assert_eq!(manager.active_count(), 3);

        // 测试按symbol查询
        let btc_subs = manager.get_subscriptions_by_symbol("BTCUSDT").await;
        assert_eq!(btc_subs.len(), 2);

        let eth_subs = manager.get_subscriptions_by_symbol("ETHUSDT").await;
        assert_eq!(eth_subs.len(), 1);

        // 测试获取所有订阅
        let all_subs = manager.get_all_subscriptions().await;
        assert_eq!(all_subs.len(), 3);

        // 测试清除所有订阅
        manager.clear().await;
        assert_eq!(manager.active_count(), 0);
    }

    #[tokio::test]
    async fn test_subscription_type_from_stream() {
        // 测试ticker
        let sub_type = SubscriptionType::from_stream("btcusdt@ticker");
        assert_eq!(sub_type, Some(SubscriptionType::Ticker));

        // 测试orderbook
        let sub_type = SubscriptionType::from_stream("btcusdt@depth");
        assert_eq!(sub_type, Some(SubscriptionType::OrderBook));

        let sub_type = SubscriptionType::from_stream("btcusdt@depth@100ms");
        assert_eq!(sub_type, Some(SubscriptionType::OrderBook));

        // 测试trades
        let sub_type = SubscriptionType::from_stream("btcusdt@trade");
        assert_eq!(sub_type, Some(SubscriptionType::Trades));

        let sub_type = SubscriptionType::from_stream("btcusdt@aggTrade");
        assert_eq!(sub_type, Some(SubscriptionType::Trades));

        // 测试kline
        let sub_type = SubscriptionType::from_stream("btcusdt@kline_1m");
        assert_eq!(sub_type, Some(SubscriptionType::Kline("1m".to_string())));

        let sub_type = SubscriptionType::from_stream("btcusdt@kline_1h");
        assert_eq!(sub_type, Some(SubscriptionType::Kline("1h".to_string())));

        // 测试markPrice
        let sub_type = SubscriptionType::from_stream("btcusdt@markPrice");
        assert_eq!(sub_type, Some(SubscriptionType::MarkPrice));

        // 测试bookTicker
        let sub_type = SubscriptionType::from_stream("btcusdt@bookTicker");
        assert_eq!(sub_type, Some(SubscriptionType::BookTicker));

        // 测试未知类型
        let sub_type = SubscriptionType::from_stream("btcusdt@unknown");
        assert_eq!(sub_type, None);
    }

    #[tokio::test]
    async fn test_subscription_send_message() {
        let manager = SubscriptionManager::new();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        // 添加订阅
        manager
            .add_subscription(
                "btcusdt@ticker".to_string(),
                "BTCUSDT".to_string(),
                SubscriptionType::Ticker,
                tx,
            )
            .await
            .unwrap();

        // 发送消息到stream
        let test_msg = serde_json::json!({
            "e": "24hrTicker",
            "s": "BTCUSDT",
            "c": "50000"
        });

        let sent = manager
            .send_to_stream("btcusdt@ticker", test_msg.clone())
            .await;
        assert!(sent);

        // 接收消息
        let received = rx.recv().await;
        assert!(received.is_some());
        assert_eq!(received.unwrap(), test_msg);
    }

    #[tokio::test]
    async fn test_subscription_send_to_symbol() {
        let manager = SubscriptionManager::new();
        let (tx1, mut rx1) = tokio::sync::mpsc::unbounded_channel();
        let (tx2, mut rx2) = tokio::sync::mpsc::unbounded_channel();

        // 为同一个symbol添加两个订阅
        manager
            .add_subscription(
                "btcusdt@ticker".to_string(),
                "BTCUSDT".to_string(),
                SubscriptionType::Ticker,
                tx1,
            )
            .await
            .unwrap();

        manager
            .add_subscription(
                "btcusdt@depth".to_string(),
                "BTCUSDT".to_string(),
                SubscriptionType::OrderBook,
                tx2,
            )
            .await
            .unwrap();

        // 发送消息到symbol的所有订阅
        let test_msg = serde_json::json!({
            "s": "BTCUSDT",
            "data": "test"
        });

        let sent_count = manager.send_to_symbol("BTCUSDT", &test_msg).await;
        assert_eq!(sent_count, 2);

        // 接收消息
        let received1 = rx1.recv().await;
        assert!(received1.is_some());
        assert_eq!(received1.unwrap(), test_msg);

        let received2 = rx2.recv().await;
        assert!(received2.is_some());
        assert_eq!(received2.unwrap(), test_msg);
    }

    #[test]
    fn test_symbol_conversion() {
        let symbol = "BTC/USDT";
        let binance_symbol = symbol.replace('/', "").to_lowercase();
        assert_eq!(binance_symbol, "btcusdt");
    }

    // ==================== MessageRouter 测试 ====================

    #[test]
    fn test_reconnect_config_default() {
        let config = ReconnectConfig::default();

        assert!(config.enabled);
        assert_eq!(config.initial_delay_ms, 1000);
        assert_eq!(config.max_delay_ms, 30000);
        assert_eq!(config.backoff_multiplier, 2.0);
        assert_eq!(config.max_attempts, 0); // 无限重连
    }

    #[test]
    fn test_reconnect_config_calculate_delay() {
        let config = ReconnectConfig::default();

        // 测试指数退避
        assert_eq!(config.calculate_delay(0), 1000); // 1s
        assert_eq!(config.calculate_delay(1), 2000); // 2s
        assert_eq!(config.calculate_delay(2), 4000); // 4s
        assert_eq!(config.calculate_delay(3), 8000); // 8s
        assert_eq!(config.calculate_delay(4), 16000); // 16s
        assert_eq!(config.calculate_delay(5), 30000); // 30s (达到最大值)
        assert_eq!(config.calculate_delay(6), 30000); // 30s (保持最大值)
    }

    #[test]
    fn test_reconnect_config_should_retry() {
        let mut config = ReconnectConfig::default();

        // 无限重连
        assert!(config.should_retry(0));
        assert!(config.should_retry(10));
        assert!(config.should_retry(100));

        // 有限重连
        config.max_attempts = 3;
        assert!(config.should_retry(0));
        assert!(config.should_retry(1));
        assert!(config.should_retry(2));
        assert!(!config.should_retry(3));
        assert!(!config.should_retry(4));

        // 禁用重连
        config.enabled = false;
        assert!(!config.should_retry(0));
        assert!(!config.should_retry(1));
    }

    #[test]
    fn test_message_router_extract_stream_name_combined() {
        // 测试组合流格式
        let message = serde_json::json!({
            "stream": "btcusdt@ticker",
            "data": {
                "e": "24hrTicker",
                "s": "BTCUSDT"
            }
        });

        let stream_name = MessageRouter::extract_stream_name(&message).unwrap();
        assert_eq!(stream_name, "btcusdt@ticker");
    }

    #[test]
    fn test_message_router_extract_stream_name_ticker() {
        // 测试ticker单流格式
        let message = serde_json::json!({
            "e": "24hrTicker",
            "s": "BTCUSDT",
            "E": 1672531200000_u64,
            "c": "16950.00",
            "h": "17100.00"
        });

        let stream_name = MessageRouter::extract_stream_name(&message).unwrap();
        assert_eq!(stream_name, "btcusdt@ticker");
    }

    #[test]
    fn test_message_router_extract_stream_name_depth() {
        // 测试深度单流格式
        let message = serde_json::json!({
            "e": "depthUpdate",
            "s": "ETHUSDT",
            "E": 1672531200000_u64,
            "U": 157,
            "u": 160
        });

        let stream_name = MessageRouter::extract_stream_name(&message).unwrap();
        assert_eq!(stream_name, "ethusdt@depth");
    }

    #[test]
    fn test_message_router_extract_stream_name_trade() {
        // 测试交易单流格式
        let message = serde_json::json!({
            "e": "trade",
            "s": "BNBUSDT",
            "E": 1672531200000_u64,
            "t": 12345
        });

        let stream_name = MessageRouter::extract_stream_name(&message).unwrap();
        assert_eq!(stream_name, "bnbusdt@trade");
    }

    #[test]
    fn test_message_router_extract_stream_name_kline() {
        // 测试K线单流格式
        let message = serde_json::json!({
            "e": "kline",
            "s": "BTCUSDT",
            "E": 1672531200000_u64,
            "k": {
                "i": "1m",
                "t": 1672531200000_u64,
                "o": "16950.00"
            }
        });

        let stream_name = MessageRouter::extract_stream_name(&message).unwrap();
        assert_eq!(stream_name, "btcusdt@kline_1m");
    }

    #[test]
    fn test_message_router_extract_stream_name_mark_price() {
        // 测试标记价格单流格式
        let message = serde_json::json!({
            "e": "markPriceUpdate",
            "s": "BTCUSDT",
            "E": 1672531200000_u64,
            "p": "16950.00"
        });

        let stream_name = MessageRouter::extract_stream_name(&message).unwrap();
        assert_eq!(stream_name, "btcusdt@markPrice");
    }

    #[test]
    fn test_message_router_extract_stream_name_book_ticker() {
        // 测试最优挂单单流格式
        let message = serde_json::json!({
            "e": "bookTicker",
            "s": "ETHUSDT",
            "E": 1672531200000_u64,
            "b": "1200.00",
            "a": "1200.50"
        });

        let stream_name = MessageRouter::extract_stream_name(&message).unwrap();
        assert_eq!(stream_name, "ethusdt@bookTicker");
    }

    #[test]
    fn test_message_router_extract_stream_name_subscription_response() {
        // 测试订阅响应（应该返回错误）
        let message = serde_json::json!({
            "result": null,
            "id": 1
        });

        let result = MessageRouter::extract_stream_name(&message);
        assert!(result.is_err());
    }

    #[test]
    fn test_message_router_extract_stream_name_error_response() {
        // 测试错误响应（应该返回错误）
        let message = serde_json::json!({
            "error": {
                "code": -1,
                "msg": "Invalid request"
            },
            "id": 1
        });

        let result = MessageRouter::extract_stream_name(&message);
        assert!(result.is_err());
    }

    #[test]
    fn test_message_router_extract_stream_name_invalid() {
        // 测试无效消息格式
        let message = serde_json::json!({
            "unknown": "data"
        });

        let result = MessageRouter::extract_stream_name(&message);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_message_router_creation() {
        let ws_url = "wss://stream.binance.com:9443/ws".to_string();
        let subscription_manager = Arc::new(SubscriptionManager::new());

        let router = MessageRouter::new(ws_url.clone(), subscription_manager);

        // 验证初始状态
        assert!(!router.is_connected());
        assert_eq!(router.ws_url, ws_url);
    }

    #[tokio::test]
    async fn test_message_router_reconnect_config() {
        let ws_url = "wss://stream.binance.com:9443/ws".to_string();
        let subscription_manager = Arc::new(SubscriptionManager::new());

        let router = MessageRouter::new(ws_url, subscription_manager);

        // 测试默认配置
        let config = router.get_reconnect_config().await;
        assert!(config.enabled);
        assert_eq!(config.initial_delay_ms, 1000);

        // 测试设置新配置
        let new_config = ReconnectConfig {
            enabled: false,
            initial_delay_ms: 2000,
            max_delay_ms: 60000,
            backoff_multiplier: 1.5,
            max_attempts: 5,
        };

        router.set_reconnect_config(new_config.clone()).await;

        let updated_config = router.get_reconnect_config().await;
        assert!(!updated_config.enabled);
        assert_eq!(updated_config.initial_delay_ms, 2000);
        assert_eq!(updated_config.max_delay_ms, 60000);
        assert_eq!(updated_config.backoff_multiplier, 1.5);
        assert_eq!(updated_config.max_attempts, 5);
    }
}
