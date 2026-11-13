//! Separated floats of any length parse exactly like the same text with its
//! separators removed.
//!
//! The standard library's `f64` parser is the reference: for every literal,
//! `parse` with separators must give the same bits (or the same failure) as
//! `str::parse::<f64>` on the cleaned text.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use numeric_lang::{Number, ParseNumericError, parse};

/// What the reference parser makes of `literal` once its separators are gone.
fn reference(literal: &str) -> Result<u64, ()> {
    literal
        .replace('_', "")
        .parse::<f64>()
        .map(f64::to_bits)
        .map_err(|_| ())
}

fn ours(literal: &str) -> Result<u64, ()> {
    match parse(literal) {
        Ok(Number::Float(f)) => Ok(f.to_bits()),
        Ok(other) => panic!("{literal:?} parsed as {other:?}"),
        Err(_) => Err(()),
    }
}

/// Insert a `_` after every `every`-th digit that is followed by another digit.
fn separate(text: &str, every: usize) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len() * 2);
    let mut run = 0usize;
    for (i, &b) in bytes.iter().enumerate() {
        out.push(b as char);
        if b.is_ascii_digit() {
            run += 1;
            let next_is_digit = bytes.get(i + 1).is_some_and(u8::is_ascii_digit);
            if run % every == 0 && next_is_digit {
                out.push('_');
            }
        } else {
            run = 0;
        }
    }
    out
}

/// The exact decimal digits of `5^n`.
fn pow5(n: usize) -> String {
    let mut digits = vec![1u8]; // little-endian
    for _ in 0..n {
        let mut carry = 0u8;
        for d in &mut digits {
            let v = *d * 5 + carry;
            *d = v % 10;
            carry = v / 10;
        }
        if carry > 0 {
            digits.push(carry);
        }
    }
    digits.iter().rev().map(|d| (b'0' + d) as char).collect()
}

/// `2^-n` written out exactly as a decimal fraction.
fn exact_pow2_neg(n: usize) -> String {
    let digits = pow5(n); // 2^-n = 5^n / 10^n
    format!("0.{}{}", "0".repeat(n - digits.len()), digits)
}

#[test]
fn test_long_separated_float_parses_like_the_plain_text() {
    // 602 bytes with a separator used to fail while the same digits parsed.
    let plain = format!("1.{}", "0".repeat(600));
    let separated = format!("1_0.{}", "0".repeat(600));
    assert_eq!(parse(&plain), Ok(Number::Float(1.0)));
    assert_eq!(parse(&separated), Ok(Number::Float(10.0)));
    assert_ne!(parse(&separated), Err(ParseNumericError::FloatTooLong));
}

#[test]
fn test_halfway_cases_round_like_the_reference() {
    // 2^-1075 sits exactly halfway between zero and the smallest subnormal.
    let half_min = exact_pow2_neg(1075);
    assert!(half_min.len() > 1000);
    let cases = [
        half_min.clone(),
        format!("{half_min}{}", "0".repeat(500)),
        format!("{half_min}{}1", "0".repeat(500)),
        // 1 + 2^-53 is halfway between 1.0 and the next double.
        format!(
            "1.00000000000000011102230246251565404236316680908203125{}",
            "0".repeat(900)
        ),
        format!(
            "1.00000000000000011102230246251565404236316680908203125{}1",
            "0".repeat(900)
        ),
        format!(
            "1.00000000000000011102230246251565404236316680908203124{}9",
            "9".repeat(900)
        ),
    ];
    for case in &cases {
        for every in [1, 3, 7, 50] {
            let literal = separate(case, every);
            assert!(literal.contains('_'));
            assert_eq!(
                ours(&literal),
                reference(&literal),
                "every {every}: {case:.60}"
            );
        }
    }
    // The sticky digit matters: these three must not all round the same way.
    let tie = separate(&cases[3], 4);
    let above = separate(&cases[4], 4);
    assert_eq!(ours(&tie), Ok(1.0f64.to_bits()));
    assert_eq!(ours(&above), Ok(1.000_000_000_000_000_2_f64.to_bits()));
}

#[test]
fn test_random_long_literals_match_the_reference() {
    // A small xorshift generator keeps the test deterministic.
    let mut state = 0x9E37_79B9_7F4A_7C15u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let pick = |n: u64, r: u64| (r % n) as usize;

    for _ in 0..3000 {
        let mut text = String::new();
        let int_len = pick(1200, next());
        let leading_zeros = pick(4, next()) == 0;
        for i in 0..int_len {
            let d = if leading_zeros && i < int_len / 2 {
                0
            } else {
                next() % 10
            };
            text.push((b'0' + d as u8) as char);
        }
        if pick(3, next()) != 0 {
            text.push('.');
            let frac_len = pick(1200, next());
            let zeros = pick(900, next());
            for i in 0..frac_len {
                let d = if i < zeros { 0 } else { next() % 10 };
                text.push((b'0' + d as u8) as char);
            }
        }
        if pick(2, next()) == 0 {
            text.push(if pick(2, next()) == 0 { 'e' } else { 'E' });
            match pick(3, next()) {
                0 => text.push('-'),
                1 => text.push('+'),
                _ => {}
            }
            let exp_len = 1 + pick(6, next());
            for _ in 0..exp_len {
                text.push((b'0' + (next() % 10) as u8) as char);
            }
        }
        let has_marker = text.contains('.') || text.contains('e') || text.contains('E');
        if !has_marker
            || !text
                .as_bytes()
                .windows(2)
                .any(|w| w[0].is_ascii_digit() && w[1].is_ascii_digit())
        {
            continue;
        }
        let literal = separate(&text, 1 + pick(40, next()));
        if !literal.contains('_') {
            continue;
        }
        assert_eq!(ours(&literal), reference(&literal), "{literal:.80}");
    }
}

#[test]
fn test_long_malformed_literals_are_refused() {
    let digits = "1_2".repeat(400);
    for bad in [
        format!("{digits}.5.5"),
        format!("{digits}e"),
        format!("{digits}e+"),
        format!("{digits}e1.5"),
        format!("{digits}.5x"),
        format!("{digits}e5e5"),
    ] {
        assert_eq!(
            parse(&bad),
            Err(ParseNumericError::MalformedFloat),
            "{bad:.40}"
        );
    }
}

#[test]
fn test_long_literals_with_extreme_exponents() {
    let digits = "9_9".repeat(400);
    for exp in [
        "e400",
        "e-400",
        "e99999999999999999999999",
        "e-99999999999999999999999",
        "e0",
    ] {
        let literal = format!("{digits}.5{exp}");
        assert_eq!(ours(&literal), reference(&literal), "{exp}");
    }
    let zeros = "0_0".repeat(400);
    assert_eq!(parse(&format!("{zeros}.0e999")), Ok(Number::Float(0.0)));
}
