use ccxt_exchanges::binance::ws::BinanceWs;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::time::Duration;
use tokio_tungstenite::tungstenite::protocol::Message;

#[tokio::test]
async fn test_binance_ws_subscribe_payload() {
    // Create a TCP listener for the mock WebSocket server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let ws_url = format!("ws://127.0.0.1:{}/ws", port);

    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws_stream = tokio_tungstenite::accept_async(stream).await.unwrap();

        // 2. Expect a subscription message
        if let Some(msg) = ws_stream.next().await {
            let msg = msg.unwrap();
            if let Message::Text(text) = msg {
                let json: Value = serde_json::from_str(&text).unwrap();

                // 3. Verify the payload format
                // Expected: {"method": "SUBSCRIBE", "params": ["btcusdt@ticker"], "id": ...}
                assert_eq!(json["method"], "SUBSCRIBE");
                assert!(json["params"].is_array());
                let params = json["params"].as_array().unwrap();
                assert_eq!(params[0], "btcusdt@ticker");
                assert!(json["id"].is_number());

                // Send response to keep connection alive if needed
                let response = serde_json::json!({
                    "result": null,
                    "id": json["id"]
                });
                ws_stream
                    .send(Message::Text(response.to_string().into()))
                    .await
                    .unwrap();
            }
        }
    });

    // 4. Client connection and subscription
    let binance_ws = BinanceWs::new(ws_url);
    binance_ws.connect().await.unwrap();

    // Give it a moment to connect
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Subscribe
    binance_ws.subscribe_ticker("BTC/USDT").await.unwrap();

    // Wait for server verification
    server_task.await.unwrap();
}
