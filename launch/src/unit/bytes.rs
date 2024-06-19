use std::{fmt, str::FromStr};

use super::Unit;

super::unit!(byte, "B", 1);
super::unit!(kilobyte, "KB", 1000);
super::unit!(megabyte, "MB", 1000 * 1000);
super::unit!(gigabyte, "GB", 1000 * 1000 * 1000);
super::unit!(kibibyte, "KiB", 1024);
super::unit!(mebibyte, "MiB", 1024 * 1024);
super::unit!(gibibyte, "GiB", 1024 * 1024 * 1024);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
pub struct Bytes(u64);

#[inline]
const fn div_round(a: u64, b: std::num::NonZeroU64) -> u64 {
    let b = b.get();
    if b == 1 {
        a
    } else {
        // `a / b` is computable and `(a % b) * 2` can not overflow since `b >= 2`.
        a / b + if (a % b) * 2 >= b { 1 } else { 0 }
    }
}

impl Bytes {
    /// Create an instance from a value and a unit.
    pub const fn new<U: Unit>(value: u64) -> Option<Self> {
        if let Some(value) = U::BASE.get().checked_mul(value) {
            Some(Self(value))
        } else {
            None
        }
    }

    /// Obtain the value in the provided unit. Performs rounding.
    pub const fn get<U: Unit>(self) -> u64 {
        div_round(self.0, U::BASE)
    }

    /// Returns an object that implements `std::fmt::Display` and formats the value in the provided unit.
    pub fn display<U: Unit>(self) -> impl fmt::Display {
        super::Display {
            value: self.get::<U>(),
            unit: U::INSTANCE,
        }
    }
}

impl fmt::Display for Bytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.display::<byte>().fmt(f)
    }
}

impl FromStr for Bytes {
    type Err = ParseBytesError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        const fn to_digit(value: u8) -> Option<u64> {
            if let b'0'..=b'9' = value {
                Some((value - b'0') as u64)
            } else {
                None
            }
        }

        const fn head_tail(value: &[u8]) -> Option<(u8, &[u8])> {
            match *value {
                [] => None,
                [head, ref tail @ ..] => Some((head, tail)),
            }
        }

        let mut input = input.as_bytes();

        let mut output = {
            let (head, tail) = head_tail(input).ok_or(ParseBytesError::Empty)?;
            let digit = to_digit(head).ok_or(ParseBytesError::InvalidDigit)?;
            input = tail;
            digit
        };

        loop {
            let (head, tail) = head_tail(input).ok_or(ParseBytesError::NoUnit)?;
            match to_digit(head) {
                Some(digit) => {
                    output = output.checked_mul(10).ok_or(ParseBytesError::PosOverflow)?;
                    output = output
                        .checked_add(digit)
                        .ok_or(ParseBytesError::PosOverflow)?;
                    input = tail;
                }
                None => {
                    break;
                }
            }
        }

        match input {
            byte::SYMBOL_BYTES => Self::new::<byte>(output),
            kilobyte::SYMBOL_BYTES => Self::new::<kilobyte>(output),
            megabyte::SYMBOL_BYTES => Self::new::<megabyte>(output),
            gigabyte::SYMBOL_BYTES => Self::new::<gigabyte>(output),
            kibibyte::SYMBOL_BYTES => Self::new::<kibibyte>(output),
            mebibyte::SYMBOL_BYTES => Self::new::<mebibyte>(output),
            gibibyte::SYMBOL_BYTES => Self::new::<gibibyte>(output),
            _ => return Err(ParseBytesError::InvalidUnit),
        }
        .ok_or(ParseBytesError::PosOverflow)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
#[non_exhaustive]
pub enum ParseBytesError {
    /// Value being parsed is empty.
    Empty,

    /// Contains an invalid digit in its context.
    InvalidDigit,

    /// Integer is too large to store in target integer type.
    PosOverflow,

    /// No unit was provided.
    NoUnit,

    /// Unit is invalid.
    InvalidUnit,
}

impl std::error::Error for ParseBytesError {}

impl fmt::Display for ParseBytesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ParseBytesError::Empty => "empty",
            ParseBytesError::InvalidDigit => "invalid digit",
            ParseBytesError::PosOverflow => "positive overflow",
            ParseBytesError::NoUnit => "no unit",
            ParseBytesError::InvalidUnit => "invalid unit",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bytes_works() {
        assert_eq!("".parse::<Bytes>(), Err(ParseBytesError::Empty));
        assert_eq!("B".parse::<Bytes>(), Err(ParseBytesError::InvalidDigit));
        assert_eq!("1".parse::<Bytes>(), Err(ParseBytesError::NoUnit));
        assert_eq!("12K".parse::<Bytes>(), Err(ParseBytesError::InvalidUnit));
        assert_eq!(
            "99999999999999999999B".parse::<Bytes>(),
            Err(ParseBytesError::PosOverflow)
        );
        assert_eq!(
            "123B".parse::<Bytes>(),
            Ok(Bytes::new::<byte>(123).unwrap())
        );
        assert_eq!(
            "123KB".parse::<Bytes>(),
            Ok(Bytes::new::<kilobyte>(123).unwrap())
        );
        assert_eq!(
            "123KiB".parse::<Bytes>(),
            Ok(Bytes::new::<kibibyte>(123).unwrap())
        );
    }

    #[test]
    fn display_works() {
        assert_eq!(
            &Bytes::new::<byte>(123)
                .unwrap()
                .display::<byte>()
                .to_string(),
            "123B"
        );
        assert_eq!(
            &Bytes::new::<kilobyte>(123)
                .unwrap()
                .display::<kilobyte>()
                .to_string(),
            "123KB"
        );
        assert_eq!(
            &Bytes::new::<kibibyte>(123)
                .unwrap()
                .display::<kibibyte>()
                .to_string(),
            "123KiB"
        );
    }

    #[test]
    fn round_on_conversion() {
        assert_eq!(Bytes::new::<byte>(700).unwrap().get::<kilobyte>(), 1);
    }
}
