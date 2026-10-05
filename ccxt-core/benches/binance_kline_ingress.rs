use std::{env, fs, hint::black_box, path::PathBuf};

use ccxt_core::ws_client::WsClient;
use criterion::{Criterion, Throughput, criterion_group, criterion_main};

fn bench_ingress(c: &mut Criterion) {
    let path = env::var_os("BINANCE_KLINE_CORPUS")
        .map(PathBuf::from)
        .expect("set BINANCE_KLINE_CORPUS to an NDJSON capture");
    let data = fs::read(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    let messages = data
        .split(|byte| *byte == b'\n')
        .map(|line| line.strip_suffix(b"\r").unwrap_or(line))
        .filter(|line| !line.iter().all(u8::is_ascii_whitespace))
        .collect::<Vec<_>>();
    assert!(!messages.is_empty(), "corpus contains no messages");
    for payload in &messages {
        WsClient::parse_ingress_json(payload).expect("fork ingress rejected corpus payload");
    }

    let total_bytes = messages.iter().map(|payload| payload.len()).sum::<usize>();
    let backend = if cfg!(feature = "sonic-ingress") {
        "sonic-ingress"
    } else {
        "serde-ingress"
    };
    eprintln!(
        "backend={backend}, messages={}, bytes={total_bytes}",
        messages.len()
    );

    let mut group = c.benchmark_group("ccxt_core_wsclient_kline_ingress");
    group.sample_size(20);
    group.throughput(Throughput::Bytes(total_bytes as u64));
    group.bench_function(backend, |b| {
        b.iter(|| {
            for payload in black_box(&messages) {
                black_box(WsClient::parse_ingress_json(payload).unwrap());
            }
        })
    });
    group.finish();
}

criterion_group!(benches, bench_ingress);
criterion_main!(benches);
