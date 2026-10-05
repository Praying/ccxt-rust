use std::{env, fs, hint::black_box, path::Path};

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use serde_json::Value;

const MAX_DEPTH: usize = 127;
const AGG_TRADE: &[u8] = br#"{"stream":"btcusdt@aggTrade","data":{"e":"aggTrade","E":1672515782136,"s":"BTCUSDT","a":12345,"p":"30000.1","q":"0.01","f":100,"l":105,"T":1672515782136,"m":true}}"#;
const KLINE_1M: &[u8] = br#"{"stream":"btcusdt@kline_1m","data":{"e":"kline","E":1672515782136,"s":"BTCUSDT","k":{"t":1672515780000,"T":1672515839999,"s":"BTCUSDT","i":"1m","f":100,"L":105,"o":"30000.0","c":"30001.0","h":"30002.0","l":"29999.0","v":"1.5","n":6,"x":false,"q":"45000.0","V":"0.8","Q":"24000.0","B":"0"}}}"#;

fn bench_fixtures(c: &mut Criterion) {
    let mut group = c.benchmark_group("binance_ingress_fixtures");
    group.sample_size(100);
    for (name, payload) in [("aggTrade", AGG_TRADE), ("kline_1m", KLINE_1M)] {
        group.throughput(Throughput::Bytes(payload.len() as u64));
        add_parser_benches(&mut group, name, payload);
    }
    group.finish();
}

fn bench_corpus(c: &mut Criterion) {
    let Ok(path) = env::var("BINANCE_CORPUS") else {
        eprintln!("BINANCE_CORPUS is unset; skipping NDJSON corpus replay");
        return;
    };
    let messages = load_ndjson(Path::new(&path));
    assert!(
        !messages.is_empty(),
        "BINANCE_CORPUS contains no JSON messages"
    );
    let bytes = messages.iter().map(Vec::len).sum::<usize>();

    // Validate compatibility once, outside all timed sections.
    for payload in &messages {
        let sonic = sonic_rs::from_slice::<Value>(payload).expect("sonic-rs rejected corpus line");
        let serde =
            serde_json::from_slice::<Value>(payload).expect("serde_json rejected corpus line");
        assert_eq!(sonic, serde, "parser value mismatch in corpus");
    }

    let mut group = c.benchmark_group("binance_ingress_corpus");
    group.sample_size(10);
    group.throughput(Throughput::Bytes(bytes as u64));
    group.bench_function("sonic-native-limit-255", |b| {
        b.iter(|| {
            for payload in black_box(&messages) {
                black_box(sonic_rs::from_slice::<Value>(payload).unwrap());
            }
        })
    });
    group.bench_function("sonic-pre-scan-limit-127", |b| {
        b.iter(|| {
            for payload in black_box(&messages) {
                validate_ingress_depth(payload).unwrap();
                black_box(sonic_rs::from_slice::<Value>(payload).unwrap());
            }
        })
    });
    group.bench_function("sonic-post-dom-limit-127", |b| {
        b.iter(|| {
            for payload in black_box(&messages) {
                let value = sonic_rs::from_slice::<Value>(payload).unwrap();
                validate_value_depth(&value).unwrap();
                black_box(value);
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

fn add_parser_benches(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    name: &str,
    payload: &[u8],
) {
    group.bench_with_input(
        BenchmarkId::new("sonic-native-limit-255", name),
        payload,
        |b, payload| b.iter(|| sonic_rs::from_slice::<Value>(black_box(payload)).unwrap()),
    );
    group.bench_with_input(
        BenchmarkId::new("sonic-pre-scan-limit-127", name),
        payload,
        |b, payload| {
            b.iter(|| {
                validate_ingress_depth(black_box(payload)).unwrap();
                sonic_rs::from_slice::<Value>(payload).unwrap()
            })
        },
    );
    group.bench_with_input(
        BenchmarkId::new("sonic-post-dom-limit-127", name),
        payload,
        |b, payload| {
            b.iter(|| {
                let value = sonic_rs::from_slice::<Value>(black_box(payload)).unwrap();
                validate_value_depth(&value).unwrap();
                value
            })
        },
    );
    group.bench_with_input(
        BenchmarkId::new("serde_json", name),
        payload,
        |b, payload| b.iter(|| serde_json::from_slice::<Value>(black_box(payload)).unwrap()),
    );
}

fn load_ndjson(path: &Path) -> Vec<Vec<u8>> {
    let data =
        fs::read(path).unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    data.split(|byte| *byte == b'\n')
        .map(|line| line.strip_suffix(b"\r").unwrap_or(line))
        .filter(|line| !line.iter().all(u8::is_ascii_whitespace))
        .map(<[u8]>::to_vec)
        .collect()
}

// Byte-for-byte equivalent to WsClient::validate_ingress_json_depth.
fn validate_ingress_depth(payload: &[u8]) -> Result<(), ()> {
    let mut depth = 0_usize;
    let mut in_string = false;
    let mut escaped = false;
    for &byte in payload {
        if in_string {
            if escaped {
                escaped = false;
            } else {
                match byte {
                    b'\\' => escaped = true,
                    b'"' => in_string = false,
                    _ => {}
                }
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b'{' | b'[' => {
                depth += 1;
                if depth > MAX_DEPTH {
                    return Err(());
                }
            }
            b'}' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    Ok(())
}

// Relies on sonic-rs's built-in 255-level parse limit, then enforces the
// fork's stricter 127-level policy iteratively without rescanning JSON text.
fn validate_value_depth(root: &Value) -> Result<(), ()> {
    let mut stack = vec![(root, 0_usize)];
    while let Some((value, depth)) = stack.pop() {
        match value {
            Value::Array(values) => {
                let child_depth = depth + 1;
                if child_depth > MAX_DEPTH {
                    return Err(());
                }
                stack.extend(values.iter().map(|value| (value, child_depth)));
            }
            Value::Object(values) => {
                let child_depth = depth + 1;
                if child_depth > MAX_DEPTH {
                    return Err(());
                }
                stack.extend(values.values().map(|value| (value, child_depth)));
            }
            _ => {}
        }
    }
    Ok(())
}

criterion_group!(benches, bench_fixtures, bench_corpus);
criterion_main!(benches);
