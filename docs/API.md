<h1 align="center">
    <img width="99" alt="Rust logo" src="https://raw.githubusercontent.com/jamesgober/rust-collection/72baabd71f00e14aa9184efcb16fa3deddda3a0a/assets/rust-logo.svg">
    <br><b>numeric-lang</b><br>
    <sub><sup>API REFERENCE</sup></sub>
</h1>
<div align="center">
    <sup>
        <a href="../README.md" title="Project Home"><b>HOME</b></a>
        <span>&nbsp;│&nbsp;</span>
        <span>API</span>
        <span>&nbsp;│&nbsp;</span>
        <a href="../CHANGELOG.md" title="Changelog"><b>CHANGELOG</b></a>
    </sup>
</div>
<br>

Parsing of numeric literals for a language front-end: radix prefixes, `_`
digit separators, and correctly-rounded decimal floats. The entire public
surface is one function and two types.

- **Version:** 1.0.0
- **MSRV:** Rust 1.85 (2024 edition)
- **`no_std`:** yes (no allocator required)
- **Stability:** stable — the surface below is frozen (see [Stability](#stability)).

## Table of Contents

- **[Stability](#stability)**
- **[Installation](#installation)**
- **[Quick Start](#quick-start)**
- **[Grammar](#grammar)**
- **[Public API](#public-api)**
  - [`parse`](#parse)
  - [`Number`](#number)
    - [`is_int`](#number-is-int)
    - [`is_float`](#number-is-float)
    - [`as_u128`](#number-as-u128)
    - [`as_f64`](#number-as-f64)
  - [`ParseNumericError`](#parsenumericerror)
- **[Feature Flags](#feature-flags)**
- **[Design Notes](#design-notes)**

<br>

## Stability

As of **1.0.0** the public API documented here is **stable and frozen**. The
crate follows [Semantic Versioning](https://semver.org):

- Nothing in the frozen surface — the [`parse`](#parse) function, the
  [`Number`](#number) enum with its variants and methods, the
  [`ParseNumericError`](#parsenumericerror) enum, and the `std` / `serde`
  feature flags — will be removed or changed in a breaking way within the `1.x`
  series. A breaking change means a new major version.
- `1.x` releases may **add** to the surface (new methods, new trait impls, new
  error variants) without breaking existing code. `ParseNumericError` is
  `#[non_exhaustive]` precisely so a new variant is a minor bump; match it with
  a wildcard arm.
- The **accepted grammar** — which strings parse and to what `Number` — is part
  of the contract. A literal that parses today will parse to the same value in
  every `1.x`. New forms may be *accepted* in a minor release, but a form that
  parses will not start being rejected, nor change its value, without a major
  bump. Which byte an error's `index` points at is a diagnostic detail, not a
  frozen guarantee.
- The **`serde` representation** of a [`Number`](#number) (an externally-tagged
  enum, `{"Int": …}` / `{"Float": …}`) is part of the contract within `1.x`.

MSRV (Rust 1.85) is treated as a compatibility surface: a raise is a minor,
documented change, never a patch.

<br>

## Installation

```toml
[dependencies]
numeric-lang = "1"

# Optional serde support (Number):
numeric-lang = { version = "1", features = ["serde"] }

# no_std (the crate needs no allocator):
numeric-lang = { version = "1", default-features = false }
```

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

<br>

## Grammar

The accepted forms, precisely:

| Form | Example | Result |
|------|---------|--------|
| Decimal integer | `1234`, `0`, `00042` | `Int` |
| Binary integer | `0b1010`, `0B1010` | `Int` |
| Octal integer | `0o755`, `0O17` | `Int` |
| Hex integer | `0xFF`, `0XdeadBEEF` | `Int` |
| Decimal float | `3.5`, `0.1`, `10.0` | `Float` |
| Float w/ exponent | `1e3`, `6.022e23`, `1E-3` | `Float` |

Rules that hold across every form:

- **Sign is not part of a literal.** A leading `+` / `-` is rejected; the
  lexer applies sign as a separate token. The exponent sign in `1e-3` is
  internal to the float grammar and is accepted.
- **A `_` separator must sit strictly between two digits.** Leading, trailing,
  doubled, and boundary-adjacent separators (next to a prefix, `.`, or `e`)
  are errors.
- **A literal is a float iff it carries a `.` or a decimal exponent.** So `10`
  is `Int(10)` and `10.0` is `Float(10.0)`. Floats are decimal only — a radix
  prefix with a `.` is rejected.
- **Integers hold an unsigned [`u128`] magnitude;** values beyond that range
  are an overflow error. Floats are IEEE-754 `f64`, parsed with correct
  rounding (the fraction/exponent grammar matches the standard library's
  `f64` parser).

The parser never panics — every rejected input is a
[`ParseNumericError`](#parsenumericerror), never a crash.

<br>

## Public API

<h3 id="parse"><code>parse</code></h3>

```rust
pub fn parse(input: &str) -> Result<Number, ParseNumericError>
```

Parse the text of one numeric literal into a [`Number`](#number).

**Parameters**

- `input` — the exact text of a single literal, as a lexer would hand it over:
  no surrounding whitespace, no trailing tokens. An empty string is an error.

**Returns**

- `Ok(Number::Int(_))` for an integer literal, `Ok(Number::Float(_))` for a
  float, or `Err(ParseNumericError)` describing the first problem found.

**Errors** — see [`ParseNumericError`](#parsenumericerror) for the full list;
`parse` is total and never panics.

**Examples**

Detecting integer vs. float:

```rust
use numeric_lang::{parse, Number};

assert_eq!(parse("10"),   Ok(Number::Int(10)));
assert_eq!(parse("10.0"), Ok(Number::Float(10.0)));
assert_eq!(parse("1e1"),  Ok(Number::Float(10.0)));
```

Reading the value out with a `match`:

```rust
use numeric_lang::{parse, Number};

let kind = match parse("0xCAFE").unwrap() {
    Number::Int(v)   => format!("int {v}"),
    Number::Float(v) => format!("float {v}"),
};
assert_eq!(kind, "int 51966");
```

Handling errors with the reported byte index:

```rust
use numeric_lang::{parse, ParseNumericError};

let err = parse("0b1210").unwrap_err();
assert_eq!(err, ParseNumericError::InvalidDigit { index: 3 });
```

<br>

<h3 id="number"><code>Number</code></h3>

```rust
pub enum Number {
    Int(u128),
    Float(f64),
}
```

A successfully parsed literal. The variant reflects the literal's *syntactic*
form, not merely its value: `10` parses to `Int(10)` and `10.0` to
`Float(10.0)`, even though the values coincide. Integers are an unsigned
[`u128`] magnitude; floats are IEEE-754 double precision.

Derives `Debug`, `Clone`, `Copy`, `PartialEq`. It is intentionally **not**
`Eq` / `Hash`, because `f64` is not: `Number::Float(f64::NAN)` is unequal to
itself. With the `serde` feature it also derives `Serialize` / `Deserialize`.

<h4 id="number-is-int"><code>Number::is_int</code></h4>

```rust
pub const fn is_int(self) -> bool
```

Returns `true` if this is an `Int`.

```rust
use numeric_lang::parse;

assert!(parse("42").unwrap().is_int());
assert!(!parse("4.2").unwrap().is_int());
```

<h4 id="number-is-float"><code>Number::is_float</code></h4>

```rust
pub const fn is_float(self) -> bool
```

Returns `true` if this is a `Float`.

```rust
use numeric_lang::parse;

assert!(parse("4.2").unwrap().is_float());
assert!(!parse("42").unwrap().is_float());
```

<h4 id="number-as-u128"><code>Number::as_u128</code></h4>

```rust
pub const fn as_u128(self) -> Option<u128>
```

Returns the integer magnitude if this is an `Int`, otherwise `None`. Never
converts a float — use it when integer-ness is a requirement, not a preference.

```rust
use numeric_lang::parse;

assert_eq!(parse("0xff").unwrap().as_u128(), Some(255));
assert_eq!(parse("2.5").unwrap().as_u128(),  None);
```

<h4 id="number-as-f64"><code>Number::as_f64</code></h4>

```rust
pub fn as_f64(self) -> f64
```

Returns the value as an `f64`. A `Float` is returned unchanged; an `Int` is
converted with the standard `as` cast — exact up to `2^53`, nearest-rounded
beyond.

```rust
use numeric_lang::parse;

assert_eq!(parse("10").unwrap().as_f64(),  10.0);
assert_eq!(parse("1.5").unwrap().as_f64(), 1.5);
```

<br>

<h3 id="parsenumericerror"><code>ParseNumericError</code></h3>

```rust
#[non_exhaustive]
pub enum ParseNumericError {
    Empty,
    InvalidDigit { index: usize },
    MisplacedSeparator { index: usize },
    MissingDigits,
    Overflow,
    MalformedFloat,
    RadixFloatUnsupported,
    FloatTooLong,
    UnexpectedChar { index: usize },
}
```

Why a literal was rejected. Variants that carry an `index` report the
zero-based byte offset of the offending character in the original input, so a
caller can underline it in a diagnostic. Implements `Display` and
[`core::error::Error`], and is `#[non_exhaustive]` (match with a wildcard arm;
new variants may appear in a minor release).

| Variant | Meaning | Example input |
|---------|---------|---------------|
| `Empty` | The input was empty. | `""` |
| `InvalidDigit { index }` | A byte is not a digit for the active radix. | `"0b12"` |
| `MisplacedSeparator { index }` | A `_` was not strictly between two digits. | `"1_"`, `"1__0"` |
| `MissingDigits` | A prefix with no digits. | `"0x"` |
| `Overflow` | Integer exceeds `u128`. | `"3402823...456"` |
| `MalformedFloat` | Malformed decimal float. | `"1e"`, `"1.2.3"` |
| `RadixFloatUnsupported` | Float syntax under a radix prefix. | `"0x1.5"` |
| `FloatTooLong` | A separated float exceeds the length cap after cleaning. | 512+ cleaned bytes |
| `UnexpectedChar { index }` | Unexpected leading character (e.g. a sign). | `"-5"`, `"+1"` |

**Examples**

Pointing at the offending byte:

```rust
use numeric_lang::{parse, ParseNumericError};

assert_eq!(parse("12_34_"),   Err(ParseNumericError::MisplacedSeparator { index: 5 }));
assert_eq!(parse("0o8"),      Err(ParseNumericError::InvalidDigit { index: 2 }));
assert_eq!(parse("0x1.5"),    Err(ParseNumericError::RadixFloatUnsupported));
```

Rendering a message for a user:

```rust
use numeric_lang::parse;

let msg = parse("1e").unwrap_err().to_string();
assert_eq!(msg, "malformed floating-point literal");
```

<br>

## Feature Flags

| Feature | Default | Description |
|---------|:-------:|-------------|
| `std`   | ✅ | Links the standard library. The crate is `no_std`-compatible with this off. |
| `serde` | ❌ | Derives `Serialize` / `Deserialize` for [`Number`](#number). |

Feature flags are additive: enabling one never removes API.

<br>

## Design Notes

**Integers are parsed in a single pass** over the bytes, accumulating into a
`u128` with `checked_mul` / `checked_add`. There is no intermediate string and
no allocation; overflow is detected exactly at the digit that would exceed the
range.

**Floats delegate the hard part.** Correctly-rounded decimal-to-`f64`
conversion is a solved, subtle problem, and the standard library already does
it well. `numeric-lang` validates separator placement and hands the cleaned
digits to the `f64` parser. When a float carries no `_`, it is parsed in place
with zero copies; only the separated case touches a fixed 512-byte stack buffer
(hence `FloatTooLong`, which no human-written literal approaches). This keeps
the float path allocation-free and `no_std` while inheriting the standard
library's round-trip correctness.

**Sign is deliberately excluded.** A literal is a magnitude. Keeping sign out
of the grammar means `parse` composes cleanly with a lexer that treats unary
minus as its own token, and it removes an entire class of ambiguity from the
surface.

<br>

<sub>Copyright &copy; 2026 <strong>James Gober</strong>.</sub>
