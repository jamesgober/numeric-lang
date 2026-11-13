//! The literal parser: prefix detection, integer accumulation, and float
//! delegation.

use crate::error::ParseNumericError;
use crate::number::Number;
use crate::radix::Radix;

/// Size, in bytes, of the stack buffer a separated float is copied into once
/// its `_` separators are removed. Literals without separators are parsed in
/// place and never touch it.
///
/// A cleaned literal that fits is copied verbatim. A longer one is rewritten
/// into the buffer in a shorter form with the same value (see
/// [`parse_long_float`]), so there is no length limit and the path stays
/// allocation-free and `no_std`.
const FLOAT_BUF_LEN: usize = 832;

/// Significant digits kept when a long separated float is rewritten.
///
/// Every `f64`, and every point exactly halfway between two adjacent `f64`
/// values, has at most 767 significant decimal digits. Keeping more than that,
/// plus one nonzero "sticky" digit standing for any nonzero digits dropped
/// after them, keeps the literal on the same side of every such point, so
/// correct rounding gives the same result as for the full literal.
const KEPT_DIGITS: usize = 800;

/// The largest decimal exponent the rewritten form carries. Anything beyond it
/// is already far outside the `f64` range (including subnormals) for a
/// mantissa of [`KEPT_DIGITS`] digits, so it rounds the same way.
const MAX_REWRITTEN_EXPONENT: i64 = 1_000_000;

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
/// radix prefix, or an unexpected leading character. This function never
/// panics on any input.
///
/// A float outside the `f64` range is not an error: like the standard
/// library's parser, it rounds to infinity when too large (`1e400`) and to
/// zero when too small (`1e-400`). Check [`f64::is_finite`] on the result if an
/// out-of-range float constant should be rejected.
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

    // Validate every separator and measure the cleaned length.
    let mut cleaned_len = 0usize;
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'_' {
            let ok = i > 0
                && i + 1 < bytes.len()
                && bytes[i - 1].is_ascii_digit()
                && bytes[i + 1].is_ascii_digit();
            if !ok {
                return Err(ParseNumericError::MisplacedSeparator { index: i });
            }
        } else {
            cleaned_len += 1;
        }
    }

    let mut buf = [0u8; FLOAT_BUF_LEN];
    let len = if cleaned_len <= FLOAT_BUF_LEN {
        let mut len = 0usize;
        for &b in bytes.iter().filter(|&&b| b != b'_') {
            buf[len] = b;
            len += 1;
        }
        len
    } else {
        parse_long_float(bytes, &mut buf)?
    };

    let cleaned =
        core::str::from_utf8(&buf[..len]).map_err(|_| ParseNumericError::MalformedFloat)?;
    cleaned
        .parse::<f64>()
        .map_err(|_| ParseNumericError::MalformedFloat)
}

/// Rewrite a separated float that is too long for the buffer as
/// `0.<digits>e<exponent>`, with the same value up to correct rounding, and
/// return the length written.
///
/// `bytes` has already had its separators validated; they are skipped here.
/// The grammar checked is the standard library's for a finite float:
/// `Digit* ['.' Digit*] [('e' | 'E') ['+' | '-'] Digit+]`, with at least one
/// mantissa digit. Leading zeros are dropped, the first [`KEPT_DIGITS`]
/// significant digits are kept, and any nonzero digit after them is folded
/// into one trailing `1`.
fn parse_long_float(
    bytes: &[u8],
    buf: &mut [u8; FLOAT_BUF_LEN],
) -> Result<usize, ParseNumericError> {
    const MALFORMED: ParseNumericError = ParseNumericError::MalformedFloat;

    // Digits start after "0.".
    let mut kept = 0usize;
    let mut sticky = false;
    let mut started = false;
    let mut mantissa_digits = 0usize;
    // The value is 0.<kept digits> * 10^decimal_exponent.
    let mut decimal_exponent: i64 = 0;
    let mut in_fraction = false;

    let mut iter = bytes.iter().copied().filter(|&b| b != b'_');
    let mut exponent_marker = false;

    for b in iter.by_ref() {
        match b {
            b'0'..=b'9' => {
                mantissa_digits += 1;
                if !started && b == b'0' {
                    // A leading zero only shifts the point when it follows it.
                    if in_fraction {
                        decimal_exponent = decimal_exponent.saturating_sub(1);
                    }
                    continue;
                }
                started = true;
                if !in_fraction {
                    decimal_exponent = decimal_exponent.saturating_add(1);
                }
                if kept < KEPT_DIGITS {
                    buf[2 + kept] = b;
                    kept += 1;
                } else if b != b'0' {
                    sticky = true;
                }
            }
            b'.' if !in_fraction => in_fraction = true,
            b'e' | b'E' => {
                exponent_marker = true;
                break;
            }
            _ => return Err(MALFORMED),
        }
    }
    if mantissa_digits == 0 {
        return Err(MALFORMED);
    }

    let mut exponent: i64 = 0;
    if exponent_marker {
        let mut negative = false;
        let mut exponent_digits = 0usize;
        for (i, b) in iter.enumerate() {
            match b {
                b'+' | b'-' if i == 0 => negative = b == b'-',
                b'0'..=b'9' => {
                    exponent_digits += 1;
                    exponent = exponent
                        .saturating_mul(10)
                        .saturating_add(i64::from(b - b'0'));
                }
                _ => return Err(MALFORMED),
            }
        }
        if exponent_digits == 0 {
            return Err(MALFORMED);
        }
        if negative {
            exponent = -exponent;
        }
    }

    if !started {
        // Every mantissa digit was zero: the value is zero whatever the exponent.
        buf[0] = b'0';
        return Ok(1);
    }

    buf[0] = b'0';
    buf[1] = b'.';
    let mut len = 2 + kept;
    if sticky {
        buf[len] = b'1';
        len += 1;
    }
    buf[len] = b'e';
    len += 1;

    let total = decimal_exponent
        .saturating_add(exponent)
        .clamp(-MAX_REWRITTEN_EXPONENT, MAX_REWRITTEN_EXPONENT);
    if total < 0 {
        buf[len] = b'-';
        len += 1;
    }
    let mut magnitude = total.unsigned_abs();
    let mut digits = [0u8; 20];
    let mut count = 0usize;
    loop {
        digits[count] = b'0' + (magnitude % 10) as u8;
        count += 1;
        magnitude /= 10;
        if magnitude == 0 {
            break;
        }
    }
    for &d in digits[..count].iter().rev() {
        buf[len] = d;
        len += 1;
    }
    Ok(len)
}

#[cfg(test)]
mod tests {
    // The tests build strings; without `std` they come from `alloc`.
    extern crate alloc;
    use alloc::string::{String, ToString};

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
    fn test_parse_long_separated_float_matches_plain_form() {
        // A separated float of any length parses like the same text without
        // its separators.
        let mut s = String::from("1_");
        s.push_str(&"0".repeat(600));
        s.push_str(".0");
        let plain = s.replace('_', "");
        assert_eq!(parse(&s), parse(&plain));
        assert_eq!(parse(&s), Ok(Number::Float(f64::INFINITY)));
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
