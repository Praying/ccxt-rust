use std::{env, fs, hint::black_box, path::PathBuf, time::Duration};

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use futures_util::StreamExt;
use serde_json::Value;
use tokio::time::{Instant, timeout_at};
use tokio_tungstenite::{connect_async, tungstenite::Message};

const DEFAULT_SYMBOLS: &str =
    "btcusdt,ethusdt,solusdt,bnbusdt,xrpusdt,dogeusdt,adausdt,trxusdt,avaxusdt,linkusdt";
const DEFAULT_MESSAGES: usize = 600;
const DEFAULT_CAPTURE_TIMEOUT_SECS: u64 = 180;

fn bench_top10_kline_1s(c: &mut Criterion) {
    let symbols = symbols_from_env();
    let target_messages = env_usize("KLINE_CAPTURE_MESSAGES", DEFAULT_MESSAGES);
    let timeout_secs = env_u64("KLINE_CAPTURE_TIMEOUT_SECS", DEFAULT_CAPTURE_TIMEOUT_SECS);
    let stream_path = combined_stream_path(&symbols);

    eprintln!("capturing {target_messages} Binance kline_1s messages");
    eprintln!("symbols: {}", symbols.join(","));
    let runtime = tokio::runtime::Runtime::new().expect("failed to create Tokio runtime");
    let messages = runtime.block_on(capture(&stream_path, target_messages, timeout_secs));
    let corpus_path = env::var_os("BINANCE_KLINE_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/binance_kline_1s_top10.ndjson"));
    let corpus = messages
        .iter()
        .flat_map(|message| message.iter().copied().chain(std::iter::once(b'\n')))
        .collect::<Vec<_>>();
    fs::write(&corpus_path, corpus)
        .unwrap_or_else(|error| panic!("failed to write {}: {error}", corpus_path.display()));
    let total_bytes = messages.iter().map(Vec::len).sum::<usize>();
    eprintln!(
        "captured {} messages ({} bytes); starting replay benchmark",
        messages.len(),
        total_bytes
    );
    eprintln!("saved corpus: {}", corpus_path.display());

    validate_corpus(&messages);

    let mut group = c.benchmark_group("binance_kline_1s_top10");
    group.sample_size(20);
    group.throughput(Throughput::Bytes(total_bytes as u64));
    group.bench_function("sonic-rs", |b| {
        b.iter(|| {
            for payload in black_box(&messages) {
                black_box(sonic_rs::from_slice::<Value>(payload).unwrap());
            }
        })
    });
    group.bench_function("serde_json", |b| {
        b.iter(|| {
            for payload in black_box(&messages) {
                black_box(serde_json::from_slice::<Value>(payload).unwrap());
            }
        })
    });
    group.finish();
}

async fn capture(stream_path: &str, target: usize, timeout_secs: u64) -> Vec<Vec<u8>> {
    let endpoint = env::var("BINANCE_WS_ENDPOINT")
        .unwrap_or_else(|_| "wss://data-stream.binance.vision".into());
    let url = format!("{}{stream_path}", endpoint.trim_end_matches('/'));
    eprintln!("endpoint: {url}");

    let (mut socket, _) = timeout_at(
        Instant::now() + Duration::from_secs(20),
        connect_async(&url),
    )
    .await
    .unwrap_or_else(|_| {
        panic!(
            "timed out connecting to {endpoint}; this host is unreachable from the current network"
        )
    })
    .unwrap_or_else(|error| panic!("failed to connect to {endpoint}: {error}"));
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    let mut messages = Vec::with_capacity(target);

    while messages.len() < target {
        let message = timeout_at(deadline, socket.next())
            .await
            .unwrap_or_else(|_| {
                panic!(
                    "capture timed out after {timeout_secs}s with {}/{} messages",
                    messages.len(),
                    target
                )
            })
            .expect("Binance closed the stream")
            .expect("Binance WebSocket read failed");
        match message {
            Message::Text(text) => messages.push(text.as_bytes().to_vec()),
            Message::Binary(data) => messages.push(data.to_vec()),
            Message::Close(frame) => panic!("Binance closed the stream: {frame:?}"),
            _ => {}
        }
    }
    messages
}

fn validate_corpus(messages: &[Vec<u8>]) {
    assert!(!messages.is_empty(), "no messages captured");
    for (index, payload) in messages.iter().enumerate() {
        let sonic = sonic_rs::from_slice::<Value>(payload)
            .unwrap_or_else(|error| panic!("sonic-rs rejected message {index}: {error}"));
        let serde = serde_json::from_slice::<Value>(payload)
            .unwrap_or_else(|error| panic!("serde_json rejected message {index}: {error}"));
        assert_eq!(sonic, serde, "parser mismatch at message {index}");
        assert_eq!(sonic["data"]["e"], "kline", "non-kline message at {index}");
        assert_eq!(sonic["data"]["k"]["i"], "1s", "non-1s kline at {index}");
    }
}

fn combined_stream_path(symbols: &[String]) -> String {
    let streams = symbols
        .iter()
        .map(|symbol| format!("{symbol}@kline_1s"))
        .collect::<Vec<_>>()
        .join("/");
    format!("/stream?streams={streams}")
}

fn symbols_from_env() -> Vec<String> {
    let raw = env::var("BINANCE_SYMBOLS").unwrap_or_else(|_| DEFAULT_SYMBOLS.into());
    let symbols = raw
        .split(',')
        .map(|symbol| symbol.trim().to_ascii_lowercase())
        .filter(|symbol| !symbol.is_empty())
        .collect::<Vec<_>>();
    assert_eq!(
        symbols.len(),
        10,
        "BINANCE_SYMBOLS must contain exactly 10 comma-separated symbols"
    );
    assert!(
        symbols
            .iter()
            .all(|symbol| symbol.chars().all(|c| c.is_ascii_alphanumeric())),
        "symbols may contain only ASCII letters and digits"
    );
    symbols
}

fn env_usize(name: &str, default: usize) -> usize {
    env::var(name)
        .map(|value| value.parse().unwrap_or_else(|_| panic!("invalid {name}")))
        .unwrap_or(default)
}

fn env_u64(name: &str, default: u64) -> u64 {
    env::var(name)
        .map(|value| value.parse().unwrap_or_else(|_| panic!("invalid {name}")))
        .unwrap_or(default)
}

criterion_group!(benches, bench_top10_kline_1s);
criterion_main!(benches);
