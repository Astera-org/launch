use std::{fmt, num::NonZeroU64};

pub mod bytes;

pub trait Unit: fmt::Display {
    const INSTANCE: Self;
    const BASE: NonZeroU64;
    const SYMBOL: &'static str;
    const SYMBOL_BYTES: &'static [u8];
}

macro_rules! __non_zero_u64_const_new {
    ($value: expr) => {{
        // Better error message in case $value is zero.
        #[allow(unused)]
        const IS_NON_ZERO: u64 = $value - 1;

        $crate::unit::__non_zero_u64_const_new_fn($value)
    }};
}

pub(crate) use __non_zero_u64_const_new;

#[doc(hidden)]
#[inline]
pub const fn __non_zero_u64_const_new_fn(value: u64) -> std::num::NonZeroU64 {
    match std::num::NonZeroU64::new(value) {
        Some(value) => value,
        None => {
            // Force compilation to fail.
            #[allow(unconditional_panic, clippy::out_of_bounds_indexing)]
            [][1]
        }
    }
}

macro_rules! unit {
    ($name:ident, $symbol:expr, $base:expr) => {
        #[allow(non_camel_case_types)]
        pub struct $name;

        impl $crate::unit::Unit for $name {
            const INSTANCE: Self = Self;
            const BASE: std::num::NonZeroU64 = $crate::unit::__non_zero_u64_const_new!($base);
            const SYMBOL: &'static str = $symbol;
            const SYMBOL_BYTES: &'static [u8] = $symbol.as_bytes();
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(Self::SYMBOL)
            }
        }
    };
}

pub(crate) use unit;

pub struct Display<V, U> {
    value: V,
    unit: U,
}

impl<V: fmt::Display, U: fmt::Display> fmt::Display for Display<V, U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.value, self.unit)
    }
}
