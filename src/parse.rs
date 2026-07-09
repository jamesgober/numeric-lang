//! The literal parser: prefix detection, integer accumulation, and float
//! delegation.

use crate::error::ParseNumericError;
use crate::number::Number;
use crate::radix::Radix;

/// Maximum length, in bytes, of a floating-point literal *after* its `_`
/// separators are removed. Literals without separators are parsed in place and
/// are not subject to this limit.
///
/// The bound only exists so the separator-stripping path can use a fixed
/// stack buffer and stay allocation-free (and `no_std`). It is far larger than
/// any human-written float: an `f64` is fully determined by ~17 significant
/// digits, and even pathological exact-decimal forms sit well under this.
const MAX_FLOAT_LITERAL_LEN: usize = 512;

/// Parse a numeric literal into a [`Number`].
///
/// The input is the exact text of one literal, as a lexer would hand it over —
/// no surrounding whitespace, no trailing tokens. The radix is taken from a
/// `0b` / `0o` / `0x` prefix (case-insensitive), defaulting to decimal. A `_`
/// is accepted as a digit separator anywhere strictly between two digits. A
/// literal is treated as floating-point when it contains a `.` or a decimal
/// exponent (`e` / `E`), and as an integer otherwise.
///
/// A leading sign is intentionally *not* accepted: a literal denotes an
/// unsigned magnitude, and sign is the lexer's concern (a separate token).
///
/// # Errors
///
/// Returns [`ParseNumericError`] describing the first problem encountered:
/// empty input, an invalid digit, a misplaced separator, missing digits, an
/// integer that overflows [`u128`], a malformed float, float syntax under a
/// radix prefix, an over-long separated float, or an unexpected leading
/// character. This function never panics on any input.
///
/// # Examples
///
/// ```
/// use numeric_lang::{parse, Number};
///
/// assert_eq!(parse("1_000_000"), Ok(Number::Int(1_000_000)));
/// assert_eq!(parse("0xFF"), Ok(Number::Int(255)));
/// assert_eq!(parse("0b1010"), Ok(Number::Int(10)));
/// assert_eq!(parse("0o755"), Ok(Number::Int(493)));
/// assert_eq!(parse("3.5"), Ok(Number::Float(3.5)));
/// assert_eq!(parse("6.022e23"), Ok(Number::Float(6.022e23)));
/// ```
#[inline]
#[must_use = "parsing a literal is pointless if the result is discarded"]
pub fn parse(input: &str) -> Result<Number, ParseNumericError> {
    let bytes = input.as_bytes();
    let Some(&first) = bytes.first() else {
        return Err(ParseNumericError::Empty);
    };
    if first == b'+' || first == b'-' {
        return Err(ParseNumericError::UnexpectedChar { index: 0 });
    }

    let (radix, offset) = detect_prefix(bytes);
    let body = &bytes[offset..];

    if radix == Radix::Decimal {
        if has_float_marker(body) {
            return parse_float(input).map(Number::Float);
        }
        parse_integer(body, radix, offset).map(Number::Int)
    } else {
        if body.contains(&b'.') {
            return Err(ParseNumericError::RadixFloatUnsupported);
        }
        parse_integer(body, radix, offset).map(Number::Int)
    }
}

/// Detect a radix prefix, returning the radix and the byte offset at which the
/// digits begin. A bare `0` (or any literal without a `0<letter>` prefix) is
/// decimal with offset `0`.
#[inline]
fn detect_prefix(bytes: &[u8]) -> (Radix, usize) {
    if bytes.len() >= 2 && bytes[0] == b'0' {
        match bytes[1] {
            b'b' | b'B' => return (Radix::Binary, 2),
            b'o' | b'O' => return (Radix::Octal, 2),
            b'x' | b'X' => return (Radix::Hexadecimal, 2),
            _ => {}
        }
    }
    (Radix::Decimal, 0)
}

/// True if a decimal body carries floating-point syntax: a `.` or an exponent
/// marker. Only ever called on decimal bodies, where `e` / `E` are never valid
/// integer digits.
#[inline]
fn has_float_marker(body: &[u8]) -> bool {
    body.iter().any(|&b| b == b'.' || b == b'e' || b == b'E')
}

/// The value of `b` as a digit in `radix`, or `None` if it is not one.
#[inline]
const fn digit_value(b: u8, radix: u32) -> Option<u32> {
    let value = match b {
        b'0'..=b'9' => (b - b'0') as u32,
        b'a'..=b'f' => (b - b'a') as u32 + 10,
        b'A'..=b'F' => (b - b'A') as u32 + 10,
        _ => return None,
    };
    if value < radix { Some(value) } else { None }
}

/// Accumulate an integer body into a [`u128`], validating digits and `_`
/// placement in a single pass. `offset` is the byte position of `body` within
/// the original input, used to report accurate error indices.
fn parse_integer(body: &[u8], radix: Radix, offset: usize) -> Result<u128, ParseNumericError> {
    let base = radix.value() as u128;
    let mut acc: u128 = 0;
    let mut saw_digit = false;
    let mut prev_was_digit = false;

    for (i, &b) in body.iter().enumerate() {
        if b == b'_' {
            if !prev_was_digit {
                return Err(ParseNumericError::MisplacedSeparator { index: offset + i });
            }
            prev_was_digit = false;
            continue;
        }

        let Some(d) = digit_value(b, radix.value()) else {
            return Err(ParseNumericError::InvalidDigit { index: offset + i });
        };

        acc = acc
            .checked_mul(base)
            .and_then(|v| v.checked_add(d as u128))
            .ok_or(ParseNumericError::Overflow)?;
        saw_digit = true;
        prev_was_digit = true;
    }

    if !saw_digit {
        return Err(ParseNumericError::MissingDigits);
    }
    // A trailing `_` leaves `prev_was_digit` false after at least one digit.
    if !prev_was_digit {
        return Err(ParseNumericError::MisplacedSeparator {
            index: offset + body.len() - 1,
        });
    }
    Ok(acc)
}

/// Parse a decimal floating-point literal with correct rounding.
///
/// The numeric grammar (fraction, exponent) is delegated to the standard
/// library's `f64` parser, which is correctly rounded. Only the `_` separator
/// handling — validated as strictly between two decimal digits — is layered on
/// top. When no separator is present the input is parsed in place with no copy.
fn parse_float(input: &str) -> Result<f64, ParseNumericError> {
    let bytes = input.as_bytes();

    if !bytes.contains(&b'_') {
        return input
            .parse::<f64>()
            .map_err(|_| ParseNumericError::MalformedFloat);
    }

    let mut buf = [0u8; MAX_FLOAT_LITERAL_LEN];
    let mut len = 0usize;

    for (i, &b) in bytes.iter().enumerate() {
        if b == b'_' {
            let ok = i > 0
                && i + 1 < bytes.len()
                && bytes[i - 1].is_ascii_digit()
                && bytes[i + 1].is_ascii_digit();
            if !ok {
                return Err(ParseNumericError::MisplacedSeparator { index: i });
            }
            continue;
        }
        if len == MAX_FLOAT_LITERAL_LEN {
            return Err(ParseNumericError::FloatTooLong);
        }
        buf[len] = b;
        len += 1;
    }

    let cleaned =
        core::str::from_utf8(&buf[..len]).map_err(|_| ParseNumericError::MalformedFloat)?;
    cleaned
        .parse::<f64>()
        .map_err(|_| ParseNumericError::MalformedFloat)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_decimal_integer_returns_value() {
        assert_eq!(parse("0"), Ok(Number::Int(0)));
        assert_eq!(parse("42"), Ok(Number::Int(42)));
        assert_eq!(parse("00042"), Ok(Number::Int(42)));
    }

    #[test]
    fn test_parse_u128_max_returns_value() {
        let s = u128::MAX.to_string();
        assert_eq!(parse(&s), Ok(Number::Int(u128::MAX)));
    }

    #[test]
    fn test_parse_decimal_overflow_returns_err() {
        // u128::MAX + 1
        assert_eq!(
            parse("340282366920938463463374607431768211456"),
            Err(ParseNumericError::Overflow)
        );
    }

    #[test]
    fn test_parse_radix_prefixes_return_values() {
        assert_eq!(parse("0b1010"), Ok(Number::Int(10)));
        assert_eq!(parse("0B1010"), Ok(Number::Int(10)));
        assert_eq!(parse("0o755"), Ok(Number::Int(493)));
        assert_eq!(parse("0O17"), Ok(Number::Int(15)));
        assert_eq!(parse("0xFF"), Ok(Number::Int(255)));
        assert_eq!(parse("0Xdead_beef"), Ok(Number::Int(0xdead_beef)));
    }

    #[test]
    fn test_parse_separators_between_digits_are_ignored() {
        assert_eq!(parse("1_000_000"), Ok(Number::Int(1_000_000)));
        assert_eq!(parse("0xFF_FF"), Ok(Number::Int(0xFFFF)));
        assert_eq!(parse("0b1010_0101"), Ok(Number::Int(0b1010_0101)));
    }

    #[test]
    fn test_parse_leading_separator_returns_err() {
        assert_eq!(
            parse("_1"),
            Err(ParseNumericError::MisplacedSeparator { index: 0 })
        );
        // Right after a prefix the first body byte is `_`.
        assert_eq!(
            parse("0x_FF"),
            Err(ParseNumericError::MisplacedSeparator { index: 2 })
        );
    }

    #[test]
    fn test_parse_trailing_separator_returns_err() {
        assert_eq!(
            parse("1_"),
            Err(ParseNumericError::MisplacedSeparator { index: 1 })
        );
        assert_eq!(
            parse("10_"),
            Err(ParseNumericError::MisplacedSeparator { index: 2 })
        );
    }

    #[test]
    fn test_parse_doubled_separator_returns_err() {
        assert_eq!(
            parse("1__0"),
            Err(ParseNumericError::MisplacedSeparator { index: 2 })
        );
    }

    #[test]
    fn test_parse_empty_returns_err() {
        assert_eq!(parse(""), Err(ParseNumericError::Empty));
    }

    #[test]
    fn test_parse_bare_prefix_returns_missing_digits() {
        assert_eq!(parse("0x"), Err(ParseNumericError::MissingDigits));
        assert_eq!(parse("0b"), Err(ParseNumericError::MissingDigits));
    }

    #[test]
    fn test_parse_invalid_digit_returns_index() {
        assert_eq!(
            parse("0b102"),
            Err(ParseNumericError::InvalidDigit { index: 4 })
        );
        assert_eq!(
            parse("0o78"),
            Err(ParseNumericError::InvalidDigit { index: 3 })
        );
        assert_eq!(
            parse("12x"),
            Err(ParseNumericError::InvalidDigit { index: 2 })
        );
    }

    #[test]
    fn test_parse_leading_sign_returns_err() {
        assert_eq!(
            parse("-1"),
            Err(ParseNumericError::UnexpectedChar { index: 0 })
        );
        assert_eq!(
            parse("+1"),
            Err(ParseNumericError::UnexpectedChar { index: 0 })
        );
        assert_eq!(
            parse("-1.5"),
            Err(ParseNumericError::UnexpectedChar { index: 0 })
        );
    }

    #[test]
    fn test_parse_decimal_float_returns_value() {
        assert_eq!(parse("3.5"), Ok(Number::Float(3.5)));
        assert_eq!(parse("0.5"), Ok(Number::Float(0.5)));
        assert_eq!(parse("10.0"), Ok(Number::Float(10.0)));
    }

    #[test]
    fn test_parse_exponent_is_float() {
        assert_eq!(parse("1e3"), Ok(Number::Float(1000.0)));
        assert_eq!(parse("1E3"), Ok(Number::Float(1000.0)));
        assert_eq!(parse("6.022e23"), Ok(Number::Float(6.022e23)));
        assert_eq!(parse("1e-3"), Ok(Number::Float(0.001)));
    }

    #[test]
    fn test_parse_float_with_separators_returns_value() {
        assert_eq!(parse("1_000.000_5"), Ok(Number::Float(1000.0005)));
        assert_eq!(parse("1_0e1_0"), Ok(Number::Float(10e10)));
    }

    #[test]
    fn test_parse_float_separator_next_to_dot_returns_err() {
        assert_eq!(
            parse("1_.5"),
            Err(ParseNumericError::MisplacedSeparator { index: 1 })
        );
        assert_eq!(
            parse("1._5"),
            Err(ParseNumericError::MisplacedSeparator { index: 2 })
        );
    }

    #[test]
    fn test_parse_malformed_float_returns_err() {
        assert_eq!(parse("1e"), Err(ParseNumericError::MalformedFloat));
        assert_eq!(parse("1.2.3"), Err(ParseNumericError::MalformedFloat));
        assert_eq!(parse("1e+"), Err(ParseNumericError::MalformedFloat));
    }

    #[test]
    fn test_parse_radix_float_returns_err() {
        assert_eq!(
            parse("0x1.5"),
            Err(ParseNumericError::RadixFloatUnsupported)
        );
        assert_eq!(
            parse("0b1.0"),
            Err(ParseNumericError::RadixFloatUnsupported)
        );
    }

    #[test]
    fn test_parse_float_never_accepts_inf_or_nan() {
        // No `.`/`e`, so these route to the integer path and fail as digits.
        assert!(matches!(
            parse("inf"),
            Err(ParseNumericError::InvalidDigit { .. })
        ));
        assert!(matches!(
            parse("nan"),
            Err(ParseNumericError::InvalidDigit { .. })
        ));
        assert!(matches!(
            parse("NaN"),
            Err(ParseNumericError::InvalidDigit { .. })
        ));
    }

    #[test]
    fn test_parse_over_long_separated_float_returns_err() {
        // A separated float over 512 bytes after cleaning trips the cap.
        let mut s = String::from("1_");
        s.push_str(&"0".repeat(600));
        s.push_str(".0");
        assert_eq!(parse(&s), Err(ParseNumericError::FloatTooLong));
    }

    #[test]
    fn test_parse_long_plain_float_is_not_length_limited() {
        // Same length, no separator: parsed in place, no cap. Rounds to a
        // finite f64 rather than erroring.
        let mut s = String::from("1.");
        s.push_str(&"0".repeat(600));
        assert_eq!(parse(&s), Ok(Number::Float(1.0)));
    }
}
