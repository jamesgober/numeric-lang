//! Criterion benchmarks for the literal parser.
//!
//! Each case isolates one path: decimal / hex integers, separated integers,
//! plain floats, and separated floats (the one path that touches the
//! stack-buffer cleaner).

use criterion::{Criterion, criterion_group, criterion_main};
use numeric_lang::parse;
use std::hint::black_box;

fn bench_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse");

    let cases = [
        ("int_decimal", "1234567890"),
        ("int_hex", "0xDEADBEEFCAFE"),
        ("int_binary", "0b1010_0101_1100_0011"),
        ("int_separated", "1_000_000_000_000"),
        ("int_u128_max", "340282366920938463463374607431768211455"),
        ("float_plain", "6.02214076e23"),
        ("float_pi", "3.141592653589793"),
        ("float_separated", "1_000_000.000_001"),
    ];

    for (name, input) in cases {
        group.bench_function(name, |b| b.iter(|| parse(black_box(input))));
    }

    group.finish();
}

criterion_group!(benches, bench_parse);
criterion_main!(benches);
