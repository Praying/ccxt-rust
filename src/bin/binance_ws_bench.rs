use std::{
    collections::VecDeque,
    io::{self, stdout},
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::Result;
use crossterm::{
    event::{Event, EventStream, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use futures_util::{SinkExt, StreamExt};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
};
use serde_json::Value;
use tokio::time::MissedTickBehavior;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tokio_util::sync::CancellationToken;

const SAMPLE_CAPACITY: usize = 4096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Feed {
    AggTrade,
    Kline,
}

impl Feed {
    const ALL: [Self; 2] = [Self::AggTrade, Self::Kline];

    fn label(self) -> &'static str {
        match self {
            Self::AggTrade => "aggTrade",
            Self::Kline => "kline_1m",
        }
    }

    fn index(self) -> usize {
        match self {
            Self::AggTrade => 0,
            Self::Kline => 1,
        }
    }
}

#[derive(Default)]
struct Samples {
    values: VecDeque<u64>,
    total: u128,
    count: u64,
    last: u64,
}

impl Samples {
    fn push(&mut self, value: u64) {
        self.last = value;
        self.total += u128::from(value);
        self.count += 1;
        if self.values.len() == SAMPLE_CAPACITY {
            self.values.pop_front();
        }
        self.values.push_back(value);
    }

    fn snapshot(&self) -> SampleView {
        let mut values: Vec<_> = self.values.iter().copied().collect();
        values.sort_unstable();
        SampleView {
            count: self.count,
            last: self.last,
            average: if self.count == 0 {
                0
            } else {
                (self.total / u128::from(self.count)) as u64
            },
            p50: percentile(&values, 50),
            p95: percentile(&values, 95),
            p99: percentile(&values, 99),
        }
    }
}

#[derive(Default)]
struct SampleView {
    count: u64,
    last: u64,
    average: u64,
    p50: u64,
    p95: u64,
    p99: u64,
}

fn percentile(sorted: &[u64], percentile: usize) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let index = ((sorted.len() - 1) * percentile).div_ceil(100);
    sorted[index]
}

struct AppState {
    started: Instant,
    status: String,
    sonic: [Samples; 2],
    serde: [Samples; 2],
    event_latency_ms: [Samples; 2],
    parser_errors: [u64; 2],
    mismatches: u64,
    messages: u64,
    latest_trade: String,
    latest_kline: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            started: Instant::now(),
            status: "starting".into(),
            sonic: Default::default(),
            serde: Default::default(),
            event_latency_ms: Default::default(),
            parser_errors: [0; 2],
            mismatches: 0,
            messages: 0,
            latest_trade: "waiting for aggTrade".into(),
            latest_kline: "waiting for kline_1m".into(),
        }
    }
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> Result<Self> {
        enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let symbol = normalize_symbol(std::env::args().nth(1).as_deref().unwrap_or("btcusdt"))?;
    let url =
        format!("wss://stream.binance.com:9443/stream?streams={symbol}@aggTrade/{symbol}@kline_1m");
    let state = Arc::new(Mutex::new(AppState::default()));
    let cancel = CancellationToken::new();
    let reader = tokio::spawn(websocket_loop(url.clone(), state.clone(), cancel.clone()));

    let guard = TerminalGuard::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;
    let ui_result = run_ui(&mut terminal, &symbol, &url, &state, &cancel).await;
    cancel.cancel();
    let _ = reader.await;
    drop(guard);
    ui_result
}

fn normalize_symbol(raw: &str) -> Result<String> {
    let symbol = raw.replace(['/', '-', '_'], "").to_ascii_lowercase();
    anyhow::ensure!(
        !symbol.is_empty() && symbol.chars().all(|c| c.is_ascii_alphanumeric()),
        "symbol must contain only ASCII letters and digits"
    );
    Ok(symbol)
}

async fn run_ui(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    symbol: &str,
    url: &str,
    state: &Arc<Mutex<AppState>>,
    cancel: &CancellationToken,
) -> Result<()> {
    let mut events = EventStream::new();
    let mut ticker = tokio::time::interval(Duration::from_millis(250));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let snapshot = snapshot(state);
                terminal.draw(|frame| draw(frame, symbol, url, &snapshot))?;
            }
            event = events.next() => {
                match event.transpose()? {
                    Some(Event::Key(key)) if key.kind == KeyEventKind::Press
                        && matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) => break,
                    Some(_) => {}
                    None => break,
                }
            }
            result = tokio::signal::ctrl_c() => {
                result?;
                break;
            }
            _ = cancel.cancelled() => break,
        }
    }
    Ok(())
}

async fn websocket_loop(url: String, state: Arc<Mutex<AppState>>, cancel: CancellationToken) {
    loop {
        set_status(&state, "connecting");
        match connect_async(&url).await {
            Ok((mut socket, _)) => {
                set_status(&state, "connected");
                loop {
                    tokio::select! {
                        _ = cancel.cancelled() => {
                            let _ = socket.close(None).await;
                            return;
                        }
                        message = socket.next() => match message {
                            Some(Ok(Message::Text(text))) => benchmark_message(text.as_bytes(), &state),
                            Some(Ok(Message::Binary(bytes))) => benchmark_message(&bytes, &state),
                            Some(Ok(Message::Ping(payload))) => {
                                if socket.send(Message::Pong(payload)).await.is_err() { break; }
                            }
                            Some(Ok(Message::Close(_))) | None => break,
                            Some(Ok(_)) => {}
                            Some(Err(error)) => {
                                set_status(&state, &format!("socket error: {error}"));
                                break;
                            }
                        }
                    }
                }
            }
            Err(error) => set_status(&state, &format!("connect error: {error}")),
        }

        if cancel.is_cancelled() {
            return;
        }
        set_status(&state, "reconnecting in 1s");
        tokio::select! {
            _ = cancel.cancelled() => return,
            _ = tokio::time::sleep(Duration::from_secs(1)) => {}
        }
    }
}

fn benchmark_message(bytes: &[u8], state: &Arc<Mutex<AppState>>) {
    let sequence = state.lock().unwrap_or_else(|e| e.into_inner()).messages;

    let (sonic, sonic_ns, serde, serde_ns) = if sequence.is_multiple_of(2) {
        let (sonic, sonic_ns) = timed_sonic(bytes);
        let (serde, serde_ns) = timed_serde(bytes);
        (sonic, sonic_ns, serde, serde_ns)
    } else {
        let (serde, serde_ns) = timed_serde(bytes);
        let (sonic, sonic_ns) = timed_sonic(bytes);
        (sonic, sonic_ns, serde, serde_ns)
    };

    let canonical = sonic.as_ref().ok().or_else(|| serde.as_ref().ok());
    let feed = canonical.and_then(classify);
    let mut app = state.lock().unwrap_or_else(|e| e.into_inner());
    app.messages += 1;
    let Some(feed) = feed else {
        app.parser_errors[0] += u64::from(sonic.is_err());
        app.parser_errors[1] += u64::from(serde.is_err());
        return;
    };
    let index = feed.index();
    if sonic.is_ok() {
        app.sonic[index].push(sonic_ns);
    } else {
        app.parser_errors[0] += 1;
    }
    if serde.is_ok() {
        app.serde[index].push(serde_ns);
    } else {
        app.parser_errors[1] += 1;
    }
    if let (Ok(left), Ok(right)) = (&sonic, &serde)
        && left != right
    {
        app.mismatches += 1;
    }
    if let Some(value) = canonical {
        update_market_view(&mut app, feed, value);
    }
}

fn timed_sonic(bytes: &[u8]) -> (sonic_rs::Result<Value>, u64) {
    let started = Instant::now();
    let result = sonic_rs::from_slice(bytes);
    (result, nanos(started.elapsed()))
}

fn timed_serde(bytes: &[u8]) -> (serde_json::Result<Value>, u64) {
    let started = Instant::now();
    let result = serde_json::from_slice(bytes);
    (result, nanos(started.elapsed()))
}

fn nanos(duration: Duration) -> u64 {
    duration.as_nanos().min(u128::from(u64::MAX)) as u64
}

fn classify(value: &Value) -> Option<Feed> {
    let event = value.get("data")?.get("e")?.as_str()?;
    match event {
        "aggTrade" => Some(Feed::AggTrade),
        "kline" => Some(Feed::Kline),
        _ => None,
    }
}

fn update_market_view(app: &mut AppState, feed: Feed, value: &Value) {
    let data = &value["data"];
    if let Some(event_ms) = data.get("E").and_then(Value::as_u64) {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        app.event_latency_ms[feed.index()].push(now_ms.saturating_sub(event_ms));
    }
    match feed {
        Feed::AggTrade => {
            app.latest_trade = format!(
                "price={}  qty={}  maker={}  id={}",
                text_field(data, "p"),
                text_field(data, "q"),
                text_field(data, "m"),
                text_field(data, "a")
            );
        }
        Feed::Kline => {
            let kline = &data["k"];
            app.latest_kline = format!(
                "O={}  H={}  L={}  C={}  volume={}  closed={}",
                text_field(kline, "o"),
                text_field(kline, "h"),
                text_field(kline, "l"),
                text_field(kline, "c"),
                text_field(kline, "v"),
                text_field(kline, "x")
            );
        }
    }
}

fn text_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .map(|v| {
            v.as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| v.to_string())
        })
        .unwrap_or_else(|| "-".into())
}

fn set_status(state: &Arc<Mutex<AppState>>, status: &str) {
    state.lock().unwrap_or_else(|e| e.into_inner()).status = status.into();
}

struct AppSnapshot {
    uptime: Duration,
    status: String,
    sonic: [SampleView; 2],
    serde: [SampleView; 2],
    latency: [SampleView; 2],
    parser_errors: [u64; 2],
    mismatches: u64,
    messages: u64,
    latest_trade: String,
    latest_kline: String,
}

fn snapshot(state: &Arc<Mutex<AppState>>) -> AppSnapshot {
    let app = state.lock().unwrap_or_else(|e| e.into_inner());
    AppSnapshot {
        uptime: app.started.elapsed(),
        status: app.status.clone(),
        sonic: [app.sonic[0].snapshot(), app.sonic[1].snapshot()],
        serde: [app.serde[0].snapshot(), app.serde[1].snapshot()],
        latency: [
            app.event_latency_ms[0].snapshot(),
            app.event_latency_ms[1].snapshot(),
        ],
        parser_errors: app.parser_errors,
        mismatches: app.mismatches,
        messages: app.messages,
        latest_trade: app.latest_trade.clone(),
        latest_kline: app.latest_kline.clone(),
    }
}

fn draw(frame: &mut ratatui::Frame<'_>, symbol: &str, url: &str, app: &AppSnapshot) {
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(9),
            Constraint::Length(6),
            Constraint::Min(5),
        ])
        .split(frame.area());

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(" {symbol} "),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(
            "{}  uptime {:02}:{:02}  messages {} ({:.1}/s)  ",
            app.status,
            app.uptime.as_secs() / 60,
            app.uptime.as_secs() % 60,
            app.messages,
            app.messages as f64 / app.uptime.as_secs_f64().max(0.001),
        )),
        Span::styled("q/Esc quit", Style::default().fg(Color::Yellow)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Binance WS parser benchmark"),
    );
    frame.render_widget(header, areas[0]);

    let mut rows = Vec::new();
    for feed in Feed::ALL {
        let i = feed.index();
        rows.push(metric_row(
            feed.label(),
            "sonic-rs",
            &app.sonic[i],
            Color::Green,
        ));
        rows.push(metric_row(
            feed.label(),
            "serde_json",
            &app.serde[i],
            Color::Blue,
        ));
    }
    let parser_table = Table::new(
        rows,
        [
            Constraint::Length(11),
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Length(11),
            Constraint::Length(11),
            Constraint::Length(11),
            Constraint::Length(11),
            Constraint::Length(11),
        ],
    )
    .header(
        Row::new([
            "feed", "parser", "count", "last", "mean", "p50", "p95", "p99",
        ])
        .style(Style::default().add_modifier(Modifier::BOLD)),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("JSON parse duration (rolling 4096 samples)"),
    );
    frame.render_widget(parser_table, areas[1]);

    let latency_rows = Feed::ALL.map(|feed| {
        let view = &app.latency[feed.index()];
        Row::new([
            Cell::from(feed.label()),
            Cell::from(view.count.to_string()),
            Cell::from(format!("{} ms", view.last)),
            Cell::from(format!("{} ms", view.p50)),
            Cell::from(format!("{} ms", view.p95)),
            Cell::from(format!("{} ms", view.p99)),
        ])
    });
    let latency_table = Table::new(
        latency_rows,
        [
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(12),
        ],
    )
    .header(
        Row::new(["feed", "count", "last", "p50", "p95", "p99"])
            .style(Style::default().add_modifier(Modifier::BOLD)),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Binance event time → local receive time (clock-skew sensitive)"),
    );
    frame.render_widget(latency_table, areas[2]);

    let details = Paragraph::new(vec![
        Line::from(format!("trade  {}", app.latest_trade)),
        Line::from(format!("kline  {}", app.latest_kline)),
        Line::from(format!(
            "errors sonic={} serde={}  value mismatches={}  parser order alternates per message",
            app.parser_errors[0], app.parser_errors[1], app.mismatches
        )),
        Line::from(format!("source  {url}")),
    ])
    .wrap(Wrap { trim: true })
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Live data / validity"),
    );
    frame.render_widget(details, areas[3]);
}

fn metric_row<'a>(feed: &'a str, parser: &'a str, view: &SampleView, color: Color) -> Row<'a> {
    Row::new([
        Cell::from(feed),
        Cell::from(parser).style(Style::default().fg(color)),
        Cell::from(view.count.to_string()),
        Cell::from(format_ns(view.last)),
        Cell::from(format_ns(view.average)),
        Cell::from(format_ns(view.p50)),
        Cell::from(format_ns(view.p95)),
        Cell::from(format_ns(view.p99)),
    ])
}

fn format_ns(value: u64) -> String {
    if value >= 1_000_000 {
        format!("{:.2} ms", value as f64 / 1_000_000.0)
    } else if value >= 1_000 {
        format!("{:.2} us", value as f64 / 1_000.0)
    } else {
        format!("{value} ns")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn benchmarks_combined_agg_trade_payload_with_both_parsers() {
        let state = Arc::new(Mutex::new(AppState::default()));
        benchmark_message(
            br#"{"stream":"btcusdt@aggTrade","data":{"e":"aggTrade","E":1672515782136,"s":"BTCUSDT","a":12345,"p":"30000.1","q":"0.01","f":100,"l":105,"T":1672515782136,"m":true}}"#,
            &state,
        );

        let app = state.lock().unwrap();
        assert_eq!(app.sonic[Feed::AggTrade.index()].count, 1);
        assert_eq!(app.serde[Feed::AggTrade.index()].count, 1);
        assert_eq!(app.mismatches, 0);
        assert_eq!(app.parser_errors, [0, 0]);
        assert!(app.latest_trade.contains("30000.1"));
    }

    #[test]
    fn benchmarks_combined_one_minute_kline_payload_with_both_parsers() {
        let state = Arc::new(Mutex::new(AppState::default()));
        benchmark_message(
            br#"{"stream":"btcusdt@kline_1m","data":{"e":"kline","E":1672515782136,"s":"BTCUSDT","k":{"t":1672515780000,"T":1672515839999,"s":"BTCUSDT","i":"1m","o":"30000.0","c":"30001.0","h":"30002.0","l":"29999.0","v":"1.5","x":false}}}"#,
            &state,
        );

        let app = state.lock().unwrap();
        assert_eq!(app.sonic[Feed::Kline.index()].count, 1);
        assert_eq!(app.serde[Feed::Kline.index()].count, 1);
        assert_eq!(app.mismatches, 0);
        assert_eq!(app.parser_errors, [0, 0]);
        assert!(app.latest_kline.contains("C=30001.0"));
    }
}
