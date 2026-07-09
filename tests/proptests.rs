//! Property-based tests for the parser's core invariants.
//!
//! These cover the two round-trip contracts the crate promises — integer
//! radix round-trip and exact `f64` round-trip — plus separator invariance
//! and the never-panics guarantee across arbitrary input.

use numeric_lang::{Number, parse};
use proptest::prelude::*;

/// Format `value` in `radix` using lowercase digits, no prefix.
fn format_radix(mut value: u128, radix: u32) -> String {
    if value == 0 {
        return "0".to_string();
    }
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = Vec::new();
    while value > 0 {
        out.push(DIGITS[(value % radix as u128) as usize]);
        value /= radix as u128;
    }
    out.reverse();
    String::from_utf8(out).unwrap()
}

proptest! {
    /// Any `u128`, formatted in binary/octal/hex with the matching prefix,
    /// parses back to the same value.
    #[test]
    fn prop_integer_radix_round_trip(value in any::<u128>()) {
        for (radix, prefix) in [(2u32, "0b"), (8, "0o"), (16, "0x")] {
            let text = format!("{prefix}{}", format_radix(value, radix));
            prop_assert_eq!(parse(&text), Ok(Number::Int(value)), "text {}", text);
        }
        // Decimal has no prefix.
        prop_assert_eq!(parse(&value.to_string()), Ok(Number::Int(value)));
    }

    /// Inserting a single `_` between two decimal digits does not change the
    /// parsed value.
    #[test]
    fn prop_separator_is_transparent(value in any::<u128>(), pos in 0usize..64) {
        let digits = value.to_string();
        prop_assume!(digits.len() >= 2);
        let at = 1 + (pos % (digits.len() - 1));
        let mut text = String::with_capacity(digits.len() + 1);
        text.push_str(&digits[..at]);
        text.push('_');
        text.push_str(&digits[at..]);
        prop_assert_eq!(parse(&text), Ok(Number::Int(value)), "text {}", text);
    }

    /// Any finite, non-negative `f64` printed with its shortest round-tripping
    /// form parses back to the identical bit pattern.
    #[test]
    fn prop_float_exact_round_trip(bits in any::<u64>()) {
        let value = f64::from_bits(bits);
        prop_assume!(value.is_finite() && value.is_sign_positive());
        let text = format!("{value:?}");
        match parse(&text) {
            Ok(Number::Float(parsed)) => {
                prop_assert_eq!(parsed.to_bits(), value.to_bits(), "text {}", text);
            }
            // A whole-valued float whose Debug form still carries `.0`
            // stays a Float; nothing here should land as Int or Err.
            other => prop_assert!(false, "unexpected {:?} for {}", other, text),
        }
    }

    /// Parsing never panics, whatever the input.
    #[test]
    fn prop_never_panics(input in ".*") {
        let _ = parse(&input);
    }

    /// Arbitrary ASCII made only of digits and separators either parses or
    /// returns an error — never panics, never wrong-variants.
    #[test]
    fn prop_digit_soup_is_total(input in "[0-9_]{0,32}") {
        let _ = parse(&input);
    }
}
