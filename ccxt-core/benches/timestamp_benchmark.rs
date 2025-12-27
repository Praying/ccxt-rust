use ccxt_core::time::{
    TimestampConversion, TimestampUtils, iso8601, milliseconds, parse_date, parse_iso8601, ymdhms,
    yyyymmdd,
};
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

// ==================== Core Timestamp Operations Benchmarks ====================

fn bench_timestamp_now(c: &mut Criterion) {
    c.bench_function("timestamp_now_ms", |b| {
        b.iter(|| black_box(TimestampUtils::now_ms()));
    });

    c.bench_function("milliseconds_wrapper", |b| {
        b.iter(|| black_box(milliseconds()));
    });
}

fn bench_timestamp_validation(c: &mut Criterion) {
    let valid_timestamps = vec![
        0i64,                     // Unix epoch
        1704110400000i64,         // 2024-01-01
        TimestampUtils::now_ms(), // Current time
        4_102_444_800_000i64,     // Year 2100 boundary
    ];

    let invalid_timestamps = vec![
        -1i64,                // Negative
        -1000000i64,          // Very negative
        4_102_444_800_001i64, // Just over year 2100
        i64::MIN,             // Minimum i64
    ];

    c.bench_function("validate_timestamp_valid", |b| {
        b.iter(|| {
            for &ts in &valid_timestamps {
                let _ = TimestampUtils::validate_timestamp(black_box(ts));
            }
        });
    });

    c.bench_function("validate_timestamp_invalid", |b| {
        b.iter(|| {
            for &ts in &invalid_timestamps {
                let _ = TimestampUtils::validate_timestamp(black_box(ts));
            }
        });
    });
}

fn bench_timestamp_conversion(c: &mut Criterion) {
    let timestamps_ms = vec![
        1704110400000i64, // 2024-01-01
        1672531200000i64, // 2023-01-01
        1640995200000i64, // 2022-01-01
        1609459200000i64, // 2021-01-01
        1577836800000i64, // 2020-01-01
    ];

    c.bench_function("seconds_to_ms", |b| {
        b.iter(|| {
            for &ts_ms in &timestamps_ms {
                let ts_s = ts_ms / 1000;
                let _ = TimestampUtils::seconds_to_ms(black_box(ts_s));
            }
        });
    });

    c.bench_function("ms_to_seconds", |b| {
        b.iter(|| {
            for &ts_ms in &timestamps_ms {
                let _ = TimestampUtils::ms_to_seconds(black_box(ts_ms));
            }
        });
    });
}

fn bench_timestamp_parsing(c: &mut Criterion) {
    let timestamp_strings = vec![
        "1704110400000",  // Integer milliseconds
        "1704110400.123", // Decimal seconds
        "1704110400.000", // Decimal seconds (no fractional)
        "1.7041104e12",   // Scientific notation
    ];

    c.bench_function("parse_timestamp_strings", |b| {
        b.iter(|| {
            for ts_str in &timestamp_strings {
                let _ = TimestampUtils::parse_timestamp(black_box(ts_str));
            }
        });
    });

    let invalid_strings = vec!["invalid", "", "not_a_number", "12345abc", "NaN", "Infinity"];

    c.bench_function("parse_timestamp_invalid_strings", |b| {
        b.iter(|| {
            for ts_str in &invalid_strings {
                let _ = TimestampUtils::parse_timestamp(black_box(ts_str));
            }
        });
    });
}

// ==================== Date Formatting Benchmarks ====================

fn bench_iso8601_formatting(c: &mut Criterion) {
    let timestamps = vec![
        1704110400000i64,         // 2024-01-01 12:00:00
        1704110400123i64,         // 2024-01-01 12:00:00.123
        1704110400999i64,         // 2024-01-01 12:00:00.999
        0i64,                     // Unix epoch
        TimestampUtils::now_ms(), // Current time
    ];

    c.bench_function("format_iso8601", |b| {
        b.iter(|| {
            for &ts in &timestamps {
                let _ = TimestampUtils::format_iso8601(black_box(ts));
            }
        });
    });

    c.bench_function("iso8601_wrapper", |b| {
        b.iter(|| {
            for &ts in &timestamps {
                let _ = iso8601(black_box(ts));
            }
        });
    });
}

fn bench_date_formatting_functions(c: &mut Criterion) {
    let timestamp = 1704110400000i64; // 2024-01-01 12:00:00

    let mut group = c.benchmark_group("date_formatting");

    group.bench_function("ymdhms_default", |b| {
        b.iter(|| {
            let _ = ymdhms(black_box(timestamp), None);
        });
    });

    group.bench_function("ymdhms_custom_separator", |b| {
        b.iter(|| {
            let _ = ymdhms(black_box(timestamp), Some("T"));
        });
    });

    group.bench_function("yyyymmdd_default", |b| {
        b.iter(|| {
            let _ = yyyymmdd(black_box(timestamp), None);
        });
    });

    group.bench_function("yyyymmdd_custom_separator", |b| {
        b.iter(|| {
            let _ = yyyymmdd(black_box(timestamp), Some("/"));
        });
    });

    group.finish();
}

// ==================== Date Parsing Benchmarks ====================

fn bench_date_parsing(c: &mut Criterion) {
    let date_strings = vec![
        "2024-01-01T12:00:00.000Z", // ISO 8601 with millis
        "2024-01-01T12:00:00Z",     // ISO 8601 without millis
        "2024-01-01 12:00:00",      // Space separated
        "2024-01-01T12:00:00.389",  // Without timezone
        "2024-01-01T12:00:00",      // Basic ISO format
    ];

    c.bench_function("parse_date_various_formats", |b| {
        b.iter(|| {
            for date_str in &date_strings {
                let _ = parse_date(black_box(date_str));
            }
        });
    });

    let iso8601_strings = vec![
        "2024-01-01T12:00:00.000Z",
        "2024-01-01T12:00:00+00:00",
        "2024-01-01 12:00:00.389",
        "2024-01-01T12:00:00",
    ];

    c.bench_function("parse_iso8601_formats", |b| {
        b.iter(|| {
            for date_str in &iso8601_strings {
                let _ = parse_iso8601(black_box(date_str));
            }
        });
    });
}

// ==================== Migration Support Benchmarks ====================

#[allow(deprecated)]
fn bench_migration_conversions(c: &mut Criterion) {
    let u64_timestamps = vec![
        1704110400000u64,
        1672531200000u64,
        1640995200000u64,
        0u64,
        (i64::MAX as u64) - 1000, // Near overflow boundary
    ];

    let i64_timestamps = vec![
        1704110400000i64,
        1672531200000i64,
        1640995200000i64,
        0i64,
        i64::MAX - 1000, // Near maximum
    ];

    c.bench_function("u64_to_i64_conversion", |b| {
        b.iter(|| {
            for &ts in &u64_timestamps {
                let _ = TimestampUtils::u64_to_i64(black_box(ts));
            }
        });
    });

    c.bench_function("i64_to_u64_conversion", |b| {
        b.iter(|| {
            for &ts in &i64_timestamps {
                let _ = TimestampUtils::i64_to_u64(black_box(ts));
            }
        });
    });

    // Test Option<u64> to Option<i64> conversion
    let option_u64_timestamps: Vec<Option<u64>> = vec![
        Some(1704110400000u64),
        Some(1672531200000u64),
        None,
        Some(0u64),
    ];

    c.bench_function("option_u64_to_i64_conversion", |b| {
        b.iter(|| {
            for ts_opt in &option_u64_timestamps {
                let _ = ts_opt.to_i64();
            }
        });
    });
}

// ==================== Day Boundary Operations Benchmarks ====================

fn bench_day_operations(c: &mut Criterion) {
    let timestamps = vec![
        1704110400000i64,         // 2024-01-01 12:00:00 (middle of day)
        1704067200000i64,         // 2024-01-01 00:00:00 (start of day)
        1704153599999i64,         // 2024-01-01 23:59:59.999 (end of day)
        TimestampUtils::now_ms(), // Current time
    ];

    c.bench_function("start_of_day", |b| {
        b.iter(|| {
            for &ts in &timestamps {
                let _ = TimestampUtils::start_of_day(black_box(ts));
            }
        });
    });

    c.bench_function("end_of_day", |b| {
        b.iter(|| {
            for &ts in &timestamps {
                let _ = TimestampUtils::end_of_day(black_box(ts));
            }
        });
    });
}

// ==================== Round Trip Performance Benchmarks ====================

fn bench_round_trip_operations(c: &mut Criterion) {
    let timestamps = vec![
        1704110400000i64,
        1672531200000i64,
        TimestampUtils::now_ms(),
        0i64,
    ];

    c.bench_function("timestamp_to_iso8601_to_timestamp", |b| {
        b.iter(|| {
            for &ts in &timestamps {
                let iso_str = TimestampUtils::format_iso8601(black_box(ts)).unwrap();
                let _ = parse_date(&iso_str);
            }
        });
    });

    c.bench_function("timestamp_validation_round_trip", |b| {
        b.iter(|| {
            for &ts in &timestamps {
                let validated = TimestampUtils::validate_timestamp(black_box(ts)).unwrap();
                let _ = TimestampUtils::validate_timestamp(validated);
            }
        });
    });
}

// ==================== Bulk Operations Benchmarks ====================

fn bench_bulk_operations(c: &mut Criterion) {
    // Generate 1000 timestamps
    let base_timestamp = 1704110400000i64;
    let bulk_timestamps: Vec<i64> = (0..1000)
        .map(|i| base_timestamp + (i * 60000)) // Every minute for ~16 hours
        .collect();

    let mut group = c.benchmark_group("bulk_operations");

    group.bench_function("validate_1000_timestamps", |b| {
        b.iter(|| {
            for &ts in &bulk_timestamps {
                let _ = TimestampUtils::validate_timestamp(black_box(ts));
            }
        });
    });

    group.bench_function("format_1000_iso8601", |b| {
        b.iter(|| {
            for &ts in &bulk_timestamps {
                let _ = TimestampUtils::format_iso8601(black_box(ts));
            }
        });
    });

    group.bench_function("convert_1000_ms_to_seconds", |b| {
        b.iter(|| {
            for &ts in &bulk_timestamps {
                let _ = TimestampUtils::ms_to_seconds(black_box(ts));
            }
        });
    });

    group.finish();
}

// ==================== Memory Allocation Benchmarks ====================

fn bench_memory_allocations(c: &mut Criterion) {
    let timestamp = 1704110400000i64;

    c.bench_function("iso8601_string_allocation", |b| {
        b.iter(|| {
            // This will allocate a new String each time
            let _ = TimestampUtils::format_iso8601(black_box(timestamp));
        });
    });

    c.bench_function("ymdhms_string_allocation", |b| {
        b.iter(|| {
            // This will allocate a new String each time
            let _ = ymdhms(black_box(timestamp), None);
        });
    });

    // Test parsing which involves string processing
    let date_string = "2024-01-01T12:00:00.000Z";
    c.bench_function("parse_date_string_processing", |b| {
        b.iter(|| {
            let _ = parse_date(black_box(date_string));
        });
    });
}

// ==================== Edge Case Performance Benchmarks ====================

fn bench_edge_cases(c: &mut Criterion) {
    let edge_case_timestamps = vec![
        0i64,                             // Unix epoch
        1i64,                             // Just after epoch
        i64::MAX,                         // Maximum i64 (will fail validation)
        TimestampUtils::YEAR_2100_MS,     // Year 2100 boundary
        TimestampUtils::YEAR_2100_MS - 1, // Just before year 2100
        -1i64,                            // Just before epoch (invalid)
    ];

    c.bench_function("edge_case_validation", |b| {
        b.iter(|| {
            for &ts in &edge_case_timestamps {
                let _ = TimestampUtils::validate_timestamp(black_box(ts));
            }
        });
    });

    c.bench_function("edge_case_formatting", |b| {
        b.iter(|| {
            for &ts in &edge_case_timestamps {
                if TimestampUtils::is_reasonable_timestamp(ts) {
                    let _ = TimestampUtils::format_iso8601(black_box(ts));
                }
            }
        });
    });
}

// Configure benchmark groups
criterion_group!(
    name = timestamp_benches;
    config = Criterion::default()
        .sample_size(50)
        .measurement_time(std::time::Duration::from_secs(5));
    targets =
        // Core operations
        bench_timestamp_now,
        bench_timestamp_validation,
        bench_timestamp_conversion,
        bench_timestamp_parsing,

        // Formatting operations
        bench_iso8601_formatting,
        bench_date_formatting_functions,

        // Parsing operations
        bench_date_parsing,

        // Migration support
        bench_migration_conversions,

        // Day operations
        bench_day_operations,

        // Round trip operations
        bench_round_trip_operations,

        // Bulk operations
        bench_bulk_operations,

        // Memory allocations
        bench_memory_allocations,

        // Edge cases
        bench_edge_cases
);

criterion_main!(timestamp_benches);
