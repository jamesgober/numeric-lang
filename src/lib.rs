//! Correct, allocation-free parsing of numeric literals.
//!
//! `numeric-lang` turns the text of a single numeric literal — as a lexer
//! hands it over — into a typed [`Number`]. It covers the three things a
//! language front-end actually needs and nothing it doesn't:
//!
//! - **Radixes.** A `0b` / `0o` / `0x` prefix (case-insensitive) selects
//!   binary, octal, or hexadecimal; no prefix is decimal.
//! - **Digit separators.** A `_` is accepted anywhere strictly between two
//!   digits, so `1_000_000` and `0xFF_FF` parse to the same values as their
//!   un-separated forms.
//! - **Exact float round-trip.** Decimal floating-point literals are parsed
//!   with correct rounding, so a float and its shortest decimal string map to
//!   the same `f64` bit pattern.
//!
//! # Design
//!
//! Integer parsing is a single pass over the bytes with checked arithmetic
//! into a [`u128`] — no allocation, no intermediate string. Float parsing
//! delegates the hard, correctly-rounded conversion to the standard library
//! and only layers separator handling on top; a literal with no `_` is parsed
//! in place with zero copies. The whole surface is `#![forbid(unsafe_code)]`
//! and works under `no_std`.
//!
//! A literal denotes an unsigned *magnitude*. A leading sign (`+` / `-`) is
//! rejected — sign belongs to the lexer as a separate token. A literal is
//! floating-point when it contains a `.` or a decimal exponent, and an integer
//! otherwise, so `10` is [`Number::Int`] while `10.0` is [`Number::Float`].
//!
//! # Quick start
//!
//! ```
//! use numeric_lang::{parse, Number};
//!
//! assert_eq!(parse("1_000_000"), Ok(Number::Int(1_000_000)));
//! assert_eq!(parse("0xFF"),      Ok(Number::Int(255)));
//! assert_eq!(parse("6.022e23"),  Ok(Number::Float(6.022e23)));
//!
//! match parse("0b1010").unwrap() {
//!     Number::Int(v) => assert_eq!(v, 10),
//!     Number::Float(_) => unreachable!(),
//! }
//! ```
//!
//! # Errors
//!
//! [`parse`] returns a [`ParseNumericError`] describing the first problem, with
//! a byte index where one applies. It never panics, on any input — malformed
//! literals are values, not crashes.
//!
//! ```
//! use numeric_lang::{parse, ParseNumericError};
//!
//! assert_eq!(parse("0b12"), Err(ParseNumericError::InvalidDigit { index: 3 }));
//! assert_eq!(parse("1_"),   Err(ParseNumericError::MisplacedSeparator { index: 1 }));
//! assert_eq!(parse("-5"),   Err(ParseNumericError::UnexpectedChar { index: 0 }));
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]
#![deny(clippy::unwrap_used, clippy::expect_used)]
#![deny(clippy::todo, clippy::unimplemented, clippy::unreachable)]
#![forbid(unsafe_code)]

mod error;
mod number;
mod parse;
mod radix;

pub use crate::error::ParseNumericError;
pub use crate::number::Number;
pub use crate::parse::parse;
