//! End-to-end tests exercising the public surface as a consumer would.

use numeric_lang::{Number, ParseNumericError, parse};

#[test]
fn test_public_surface_round_trips_each_radix() {
    assert_eq!(parse("255"), Ok(Number::Int(255)));
    assert_eq!(parse("0xff"), Ok(Number::Int(255)));
    assert_eq!(parse("0o377"), Ok(Number::Int(255)));
    assert_eq!(parse("0b1111_1111"), Ok(Number::Int(255)));
}

#[test]
fn test_number_accessors_agree_with_variant() {
    let int = parse("0x2a").unwrap();
    assert!(int.is_int());
    assert_eq!(int.as_u128(), Some(42));
    assert_eq!(int.as_f64(), 42.0);

    let float = parse("42.0").unwrap();
    assert!(float.is_float());
    assert_eq!(float.as_u128(), None);
    assert_eq!(float.as_f64(), 42.0);
}

#[test]
fn test_error_display_is_human_readable() {
    let err = parse("0b12").unwrap_err();
    assert_eq!(err, ParseNumericError::InvalidDigit { index: 3 });
    assert!(err.to_string().contains("invalid digit"));

    let err = parse("").unwrap_err();
    assert_eq!(err.to_string(), "empty numeric literal");
}

#[test]
fn test_error_implements_std_error() {
    fn assert_error<E: std::error::Error>(_: &E) {}
    let err = parse("nope").unwrap_err();
    assert_error(&err);
}

#[test]
fn test_zero_variants_all_parse_to_zero() {
    for input in ["0", "0x0", "0o0", "0b0", "0.0", "0e0"] {
        let parsed = parse(input).unwrap();
        assert_eq!(parsed.as_f64(), 0.0, "input {input:?}");
    }
}

#[test]
fn test_exact_float_round_trip_for_known_hard_values() {
    // These are classic values where a naive parser loses the last bit.
    for input in ["0.1", "0.2", "0.3", "2.2250738585072014e-308"] {
        let expected: f64 = input.parse().unwrap();
        assert_eq!(parse(input), Ok(Number::Float(expected)), "input {input}");
    }
}
