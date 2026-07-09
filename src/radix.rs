//! The numeric base a literal is written in.
//!
//! `Radix` is an internal detail: [`parse`](crate::parse) detects it from the
//! literal's prefix and uses it while accumulating digits, but it does not
//! appear in the public surface. It is deliberately not exported — a future
//! minor release may promote it if a caller-facing need arises, which is an
//! additive, non-breaking change; exporting it now and retracting it would not
//! be.

/// The base an integer literal is expressed in.
///
/// Detected from a two-character prefix: `0b` (binary), `0o` (octal), `0x`
/// (hexadecimal), each case-insensitive on the letter. No prefix is decimal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Radix {
    /// Base 2, prefix `0b` / `0B`.
    Binary,
    /// Base 8, prefix `0o` / `0O`.
    Octal,
    /// Base 10, no prefix.
    Decimal,
    /// Base 16, prefix `0x` / `0X`.
    Hexadecimal,
}

impl Radix {
    /// The numeric base as an integer: `2`, `8`, `10`, or `16`.
    #[inline]
    pub(crate) const fn value(self) -> u32 {
        match self {
            Self::Binary => 2,
            Self::Octal => 8,
            Self::Decimal => 10,
            Self::Hexadecimal => 16,
        }
    }
}
