use criterion::{Criterion, criterion_group, criterion_main};
use datadog_cli::utils::{parse_time, truncate_stack_trace};
use std::hint::black_box;

fn bench_parse_time(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_time");

    group.bench_function("unix_timestamp", |b| {
        b.iter(|| parse_time(black_box("1704067200")))
    });

    group.bench_function("natural_language", |b| {
        b.iter(|| parse_time(black_box("1 hour ago")))
    });

    group.bench_function("iso8601", |b| {
        b.iter(|| parse_time(black_box("2024-01-01T00:00:00Z")))
    });

    group.finish();
}

fn bench_truncate_stack_trace(c: &mut Criterion) {
    let stack = (0..100)
        .map(|i| format!("  at frame_{i} (module.rs:{i})"))
        .collect::<Vec<_>>()
        .join("\n");

    c.bench_function("truncate_stack_trace_100_lines", |b| {
        b.iter(|| truncate_stack_trace(black_box(&stack), black_box(10)))
    });
}

criterion_group!(benches, bench_parse_time, bench_truncate_stack_trace);
criterion_main!(benches);
