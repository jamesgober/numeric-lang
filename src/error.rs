//! The error type returned when a numeric literal cannot be interpreted.

use core::fmt;

/// The reason a numeric literal failed to parse.
///
/// Every variant identifies a distinct, actionable failure. Where a specific
/// byte is at fault, the variant carries its zero-based `index` into the
/// original input so a caller can point at it in a diagnostic.
///
/// The type is [`#[non_exhaustive]`](https://doc.rust-lang.org/reference/attributes/type_system.html#the-non_exhaustive-attribute):
/// future minor releases may add variants for newly distinguished failures
/// without a major version bump. Match with a wildcard arm.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseNumericError {
    /// The input was empty. There is nothing to parse.
    Empty,

    /// A byte that is not a valid digit for the active radix appeared at
    /// `index`. For example, `'2'` in a binary literal, or any letter in a
    /// decimal integer.
    InvalidDigit {
        /// Byte offset of the offending character in the original input.
        index: usize,
    },

    /// A `_` digit separator at `index` was not placed strictly between two
    /// digits. Leading, trailing, doubled, and boundary-adjacent separators
    /// (next to a prefix, `.`, or exponent marker) all fail here.
    MisplacedSeparator {
        /// Byte offset of the offending `_` in the original input.
        index: usize,
    },

    /// The literal carried a radix prefix or sign but no actual digits, such
    /// as a bare `0x`.
    MissingDigits,

    /// The integer value did not fit in a [`u128`]. Parsing stops at the point
    /// the running value would exceed [`u128::MAX`]; the exact overflowing
    /// digit is not reported because the whole literal is out of range.
    Overflow,

    /// A decimal floating-point literal was syntactically malformed — for
    /// instance an exponent with no digits (`1e`) or a stray `.` (`1.2.3`).
    MalformedFloat,

    /// A radix-prefixed literal (`0x`, `0o`, `0b`) used floating-point syntax
    /// (a `.`), which this crate does not support. Only decimal floats exist.
    RadixFloatUnsupported,

    /// A floating-point literal containing `_` separators exceeded the
    /// maximum supported length after the separators were removed. See the
    /// crate docs for the exact limit. Literals without separators are not
    /// length-limited.
    FloatTooLong,

    /// An unexpected character appeared where a digit or radix prefix was
    /// expected — most commonly a leading sign (`+` / `-`), which is not part
    /// of a literal's magnitude.
    UnexpectedChar {
        /// Byte offset of the unexpected character in the original input.
        index: usize,
    },
}

impl fmt::Display for ParseNumericError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("empty numeric literal"),
            Self::InvalidDigit { index } => {
                write!(f, "invalid digit for radix at byte {index}")
            }
            Self::MisplacedSeparator { index } => {
                write!(f, "misplaced `_` separator at byte {index}")
            }
            Self::MissingDigits => f.write_str("numeric literal has no digits"),
            Self::Overflow => f.write_str("integer literal exceeds the u128 range"),
            Self::MalformedFloat => f.write_str("malformed floating-point literal"),
            Self::RadixFloatUnsupported => {
                f.write_str("floating-point syntax is not supported with a radix prefix")
            }
            Self::FloatTooLong => {
                f.write_str("floating-point literal is too long after removing separators")
            }
            Self::UnexpectedChar { index } => {
                write!(f, "unexpected character at byte {index}")
            }
        }
    }
}

impl core::error::Error for ParseNumericError {}
