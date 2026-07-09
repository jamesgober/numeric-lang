//! The parsed value of a numeric literal.

/// A successfully parsed numeric literal.
///
/// The variant reflects the literal's *syntactic* form, not merely its value:
/// a literal is a [`Number::Float`] when it carries a `.` or a decimal
/// exponent, and a [`Number::Int`] otherwise. So `10` parses to `Int(10)` and
/// `10.0` to `Float(10.0)`, even though the values coincide.
///
/// Integers are stored as an unsigned [`u128`] magnitude; a leading sign is
/// not part of a literal (see the crate-level documentation). Floats are IEEE
/// 754 double precision and are parsed with correct rounding, so a decimal
/// float and its shortest round-trip string map to the same bit pattern.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Number {
    /// An integer magnitude, in any supported radix.
    Int(u128),
    /// A decimal floating-point value.
    Float(f64),
}

impl Number {
    /// Returns `true` if this is an [`Int`](Number::Int).
    ///
    /// # Examples
    ///
    /// ```
    /// use numeric_lang::parse;
    ///
    /// assert!(parse("42").unwrap().is_int());
    /// assert!(!parse("4.2").unwrap().is_int());
    /// ```
    #[inline]
    #[must_use]
    pub const fn is_int(self) -> bool {
        matches!(self, Self::Int(_))
    }

    /// Returns `true` if this is a [`Float`](Number::Float).
    ///
    /// # Examples
    ///
    /// ```
    /// use numeric_lang::parse;
    ///
    /// assert!(parse("4.2").unwrap().is_float());
    /// assert!(!parse("42").unwrap().is_float());
    /// ```
    #[inline]
    #[must_use]
    pub const fn is_float(self) -> bool {
        matches!(self, Self::Float(_))
    }

    /// Returns the integer magnitude if this is an [`Int`](Number::Int),
    /// otherwise `None`.
    ///
    /// This never converts a float; use it when integer-ness is required.
    ///
    /// # Examples
    ///
    /// ```
    /// use numeric_lang::parse;
    ///
    /// assert_eq!(parse("0xff").unwrap().as_u128(), Some(255));
    /// assert_eq!(parse("2.5").unwrap().as_u128(), None);
    /// ```
    #[inline]
    #[must_use]
    pub const fn as_u128(self) -> Option<u128> {
        match self {
            Self::Int(v) => Some(v),
            Self::Float(_) => None,
        }
    }

    /// Returns the value as an [`f64`].
    ///
    /// A [`Float`](Number::Float) is returned unchanged. An [`Int`](Number::Int)
    /// is converted with the standard `as` cast: exact up to 2^53, and rounded
    /// to the nearest representable `f64` beyond that.
    ///
    /// # Examples
    ///
    /// ```
    /// use numeric_lang::parse;
    ///
    /// assert_eq!(parse("10").unwrap().as_f64(), 10.0);
    /// assert_eq!(parse("1.5").unwrap().as_f64(), 1.5);
    /// ```
    #[inline]
    #[must_use]
    pub fn as_f64(self) -> f64 {
        match self {
            Self::Int(v) => v as f64,
            Self::Float(v) => v,
        }
    }
}
