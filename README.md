<h1 align="center">
    <img width="99" alt="Rust logo" src="https://raw.githubusercontent.com/jamesgober/rust-collection/72baabd71f00e14aa9184efcb16fa3deddda3a0a/assets/rust-logo.svg">
    <br>
    <b>numeric-lang</b>
    <br>
    <sub><sup>NUMERIC LITERALS</sup></sub>
</h1>

<div align="center">
    <a href="https://crates.io/crates/numeric-lang"><img alt="Crates.io" src="https://img.shields.io/crates/v/numeric-lang"></a>
    <a href="https://crates.io/crates/numeric-lang"><img alt="Downloads" src="https://img.shields.io/crates/d/numeric-lang?color=%230099ff"></a>
    <a href="https://docs.rs/numeric-lang"><img alt="docs.rs" src="https://img.shields.io/docsrs/numeric-lang"></a>
    <a href="https://github.com/jamesgober/numeric-lang/actions"><img alt="CI" src="https://github.com/jamesgober/numeric-lang/actions/workflows/ci.yml/badge.svg"></a>
    <a href="https://github.com/rust-lang/rfcs/blob/master/text/2495-min-rust-version.md"><img alt="MSRV" src="https://img.shields.io/badge/MSRV-1.85%2B-blue"></a>
</div>

<br>

<div align="left">
    <p>
        numeric-lang is the FEAT-tier crate: correct numeric literal parsing — radixes, digit separators, and exact float round-trip. Part of the -lang language-construction family; see _strategy/LANG_COLLECTION.md for the master plan.
    </p>
    <br>
    <hr>
    <p>
        <strong>MSRV is 1.85+</strong> (Rust 2024 edition).
    </p>
    <blockquote>
        <strong>Status: stable.</strong> As of <code>1.0.0</code> the public API is frozen under Semantic Versioning; see <a href="./docs/API.md#stability"><code>docs/API.md</code></a> for the promise and <a href="./CHANGELOG.md"><code>CHANGELOG.md</code></a> for the history.
    </blockquote>
</div>

<hr>
<br>

<div align="left">
    <p>
        <strong>numeric-lang</strong> turns the text of one numeric literal — as a lexer hands it over — into a typed <a href="./docs/API.md#number"><code>Number</code></a>. It does the three things a language front-end actually needs and nothing it doesn't: <b>radix prefixes</b> (<code>0b</code> / <code>0o</code> / <code>0x</code>), <b>digit separators</b> (<code>1_000_000</code>), and <b>correctly-rounded decimal floats</b>, so a float and its shortest decimal string map to the same <code>f64</code>.
    </p>
    <p>
        Integers parse in a single pass with checked arithmetic into a <code>u128</code> — no intermediate string, no allocation. Floats delegate the subtle, correctly-rounded conversion to the standard library and layer only separator handling on top; a literal with no <code>_</code> is parsed in place with zero copies. The whole surface is <code>#![forbid(unsafe_code)]</code>, <b><code>no_std</code></b>, and needs no allocator. It never panics: every malformed literal is a <a href="./docs/API.md#parsenumericerror"><code>ParseNumericError</code></a> value with the byte offset of the fault.
    </p>
</div>

<hr>
<br>

## Performance First

Parsing a literal is a hot path in a lexer, so it is measured, not asserted. Latest local Criterion means (`cargo bench --bench bench`, Linux x86_64 / WSL2, Rust stable, release build):

| Literal | Example | Time |
|---------|---------|-----:|
| Plain float | `6.02214076e23` | ~10 ns |
| Decimal integer (10 digits) | `1234567890` | ~12 ns |
| Float, 16 significant digits | `3.141592653589793` | ~16 ns |
| Hex integer | `0xDEADBEEFCAFE` | ~19 ns |
| Separated integer | `1_000_000_000_000` | ~19 ns |
| Separated float | `1_000_000.000_001` | ~33 ns |
| `u128::MAX` (39 digits) | `340282366920938463463374607431768211455` | ~50 ns |

Numbers vary by CPU and environment; run the suite on your target to establish a baseline. No allocation occurs on any path — the separated-float case uses a fixed stack buffer, everything else works on the input bytes directly.

<br>
<hr>

## Features

- **Every radix** — binary `0b`, octal `0o`, hex `0x` (case-insensitive), and prefix-free decimal, into an unsigned `u128` magnitude.
- **Digit separators** — `_` accepted anywhere strictly between two digits, in integers and floats alike.
- **Exact float round-trip** — decimal floats parsed with correct rounding; a float and its shortest decimal string share a bit pattern.
- **Allocation-free** — integers parse in one pass; floats without separators parse in place. Nothing heap-allocates.
- **Never panics** — malformed input is a typed error carrying the offending byte index, not a crash.
- **Fully safe** — `#![forbid(unsafe_code)]`.
- **`no_std`, no allocator** — runs anywhere Rust does.
- **Property-tested** — radix round-trip, exact `f64` round-trip, separator transparency, and totality checked across randomized inputs with `proptest`.

<br>
<hr>

## Installation

```toml
[dependencies]
numeric-lang = "1"

# With serde support (Number):
numeric-lang = { version = "1", features = ["serde"] }

# no_std (needs no allocator):
numeric-lang = { version = "1", default-features = false }
```

**MSRV is 1.85+** (Rust 2024 edition).

<hr>
<br>

## Quick Start

```rust
use numeric_lang::{parse, Number};

// Radix prefixes, case-insensitive.
assert_eq!(parse("42"),     Ok(Number::Int(42)));
assert_eq!(parse("0xFF"),   Ok(Number::Int(255)));
assert_eq!(parse("0o755"),  Ok(Number::Int(493)));
assert_eq!(parse("0b1010"), Ok(Number::Int(10)));

// Digit separators are transparent.
assert_eq!(parse("1_000_000"), Ok(Number::Int(1_000_000)));

// A `.` or exponent makes it a float, parsed with correct rounding.
assert_eq!(parse("3.5"),      Ok(Number::Float(3.5)));
assert_eq!(parse("6.022e23"), Ok(Number::Float(6.022e23)));
```

### Integer or float?

A literal is a float when — and only when — it carries a `.` or a decimal
exponent. So `10` and `10.0` are distinct results even though the values match.

```rust
use numeric_lang::{parse, Number};

fn label(text: &str) -> &'static str {
    match parse(text) {
        Ok(Number::Int(_))   => "int",
        Ok(Number::Float(_)) => "float",
        Err(_)               => "invalid",
    }
}

assert_eq!(label("10"),   "int");
assert_eq!(label("10.0"), "float");
assert_eq!(label("1e9"),  "float");
assert_eq!(label("0x-1"), "invalid");
```

### Errors point at the fault

```rust
use numeric_lang::{parse, ParseNumericError};

// Byte 3 is the `2`, which is not a binary digit.
assert_eq!(parse("0b1210"), Err(ParseNumericError::InvalidDigit { index: 3 }));

// A sign is the lexer's job, not the literal's.
assert_eq!(parse("-5"), Err(ParseNumericError::UnexpectedChar { index: 0 }));

// Separators must sit between digits.
assert_eq!(parse("1__0"), Err(ParseNumericError::MisplacedSeparator { index: 2 }));
```

<hr>
<br>

## What it does not do

By design, so the surface stays small and unambiguous:

- **No sign.** A literal is an unsigned magnitude; `+` / `-` belong to the lexer as a separate token (the exponent sign in `1e-3` is internal and accepted).
- **No non-decimal floats.** `0x1.5p4`-style hex floats are rejected; floats are decimal only.
- **No suffixes.** Type suffixes like `10u8` or `1.0f32` are not parsed — strip them in the lexer first.
- **No surrounding whitespace or trailing tokens.** `parse` expects exactly one literal.

<hr>
<br>

## API Overview

For the complete reference with examples, see [`docs/API.md`](./docs/API.md).

- [`parse`](./docs/API.md#parse) — parse one literal into a `Number`.
- [`Number`](./docs/API.md#number) — the result: `Int(u128)` or `Float(f64)`, with `is_*` / `as_*` accessors.
- [`ParseNumericError`](./docs/API.md#parsenumericerror) — why a literal was rejected, with a byte index.

<br>

### Feature Flags

| Feature | Default | Description |
|---------|:-------:|-------------|
| `std`   | ✅ | Links the standard library. The crate is `no_std`-compatible with this off. |
| `serde` | ❌ | `Serialize` / `Deserialize` for `Number`. |

<hr>
<br>

## Testing

```bash
cargo test                 # unit + integration + property + doctests
cargo test --all-features  # adds the serde-gated paths
cargo bench --bench bench  # Criterion hot-path benchmarks
```

The property suite in [`tests/proptests.rs`](./tests/proptests.rs) checks the
core contracts — integer radix round-trip, exact `f64` round-trip, separator
transparency, and that parsing is total (never panics) — across randomized
inputs.

<hr>
<br>

## Cross-Platform Support

The parser is pure integer and float arithmetic with no platform-specific
code, so it behaves identically everywhere Rust runs. CI covers **Linux**,
**macOS**, and **Windows** on both stable and the 1.85 MSRV.

<hr>
<br>

## Contributing

See <a href="./REPS.md"><code>REPS.md</code></a> for the engineering standards and the definition of done. Before a PR: `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all-features` must be clean.

<br>

<div id="license">
    <h2>License</h2>
    <p>Licensed under either of</p>
    <ul>
        <li><b>Apache License, Version 2.0</b> &mdash; <a href="./LICENSE-APACHE">LICENSE-APACHE</a></li>
        <li><b>MIT License</b> &mdash; <a href="./LICENSE-MIT">LICENSE-MIT</a></li>
    </ul>
    <p>at your option.</p>
</div>

<div align="center">
  <h2></h2>
  <sup>COPYRIGHT <small>&copy;</small> 2026 <strong>James Gober <me@jamesgober.com>.</strong></sup>
</div>
