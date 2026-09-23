//! Checked conversions from PLC numeric types into native Rust types.

use super::{Float, Integer, Number, SignedInteger, UnsignedInteger};
use crate::Error;
use std::fmt;

pub fn lossy(from: impl fmt::Display, to: &'static str) -> Error {
    Error::LossyConversion {
        from: from.to_string(),
        to,
    }
}

#[macro_export]
macro_rules! signed_to_int {
    ($($t:ty),* $(,)?) => {$(
        impl TryFrom<SignedInteger> for $t {
            type Error = Error;

            fn try_from(s: SignedInteger) -> Result<Self, Self::Error> {
                let v: i64 = s.into();
                <$t>::try_from(v).map_err(|_| lossy(v, stringify!($t)))
            }
        }
    )*};
}

signed_to_int!(i8, i16, i32, u8, u16, u32, u64);

macro_rules! unsigned_to_int {
    ($($t:ty),* $(,)?) => {$(
        impl TryFrom<UnsignedInteger> for $t {
            type Error = Error;

            fn try_from(u: UnsignedInteger) -> Result<Self, Self::Error> {
                let v: u64 = u.into();
                <$t>::try_from(v).map_err(|_| lossy(v, stringify!($t)))
            }
        }
    )*};
}
unsigned_to_int!(i8, i16, i32, i64, u8, u16, u32);

macro_rules! signed_to_float {
    ($($t:ty),* $(,)?) => {$(
        impl TryFrom<SignedInteger> for $t {
            type Error = Error;

            fn try_from(s: SignedInteger) -> Result<Self, Self::Error> {
                let v: i64 = s.into();
                let f = v as $t;
                if f as i128 == v as i128 { Ok(f) }
                else { Err(lossy(v, stringify!($t))) }
            }
        }
    )*};
}
signed_to_float!(f32, f64);

macro_rules! unsigned_to_float {
    ($($t:ty),* $(,)?) => {$(
        impl TryFrom<UnsignedInteger> for $t {
            type Error = Error;

            fn try_from(u: UnsignedInteger) -> Result<Self, Self::Error> {
                let v: u64 = u.into();
                let f = v as $t;
                if f as u128 == v as u128 { Ok(f) }
                else { Err(lossy(v, stringify!($t))) }
            }
        }
    )*};
}
unsigned_to_float!(f32, f64);

macro_rules! float_to_int {
    ($($t:ty),* $(,)?) => {$(
        impl TryFrom<Float> for $t {
            type Error = Error;

            fn try_from(f: Float) -> Result<Self, Self::Error> {
                let v = f64::from(f);
                if !v.is_finite() || v.fract() != 0.0 {
                    return Err(lossy(v, stringify!($t)));
                }
                let lo = <$t>::MIN as f64;
                let hi = <$t>::MAX as f64 + 1.0;
                if !(lo..hi).contains(&v) {
                    return Err(lossy(v, stringify!($t)));
                }
                Ok(v as $t)
            }
        }
    )*};
}
float_to_int!(i8, i16, i32, i64, u8, u16, u32, u64);

impl TryFrom<Float> for f32 {
    type Error = Error;

    fn try_from(f: Float) -> Result<Self, Self::Error> {
        match f {
            Float::Real(n) => Ok(n),
            Float::LReal(n) => {
                let narrowed = n as f32;
                if n.is_nan() || narrowed as f64 == n {
                    Ok(narrowed)
                } else {
                    Err(lossy(n, "f32"))
                }
            }
        }
    }
}

macro_rules! integer_to_prim {
    ($($t:ty),* $(,)?) => {$(
        impl TryFrom<Integer> for $t {
            type Error = Error;

            fn try_from(i: Integer) -> Result<Self, Self::Error> {
                match i {
                    Integer::Signed(s)   => <$t>::try_from(s),
                    Integer::Unsigned(u) => <$t>::try_from(u),
                }
            }
        }
    )*};
}
integer_to_prim!(i8, i16, i32, u8, u16, u32, f32, f64);

impl TryFrom<Integer> for i64 {
    type Error = Error;

    fn try_from(i: Integer) -> Result<Self, Error> {
        match i {
            Integer::Signed(s) => Ok(i64::from(s)),
            Integer::Unsigned(u) => i64::try_from(u),
        }
    }
}

impl TryFrom<Integer> for u64 {
    type Error = Error;

    fn try_from(i: Integer) -> Result<Self, Error> {
        match i {
            Integer::Signed(s) => u64::try_from(s),
            Integer::Unsigned(u) => Ok(u64::from(u)),
        }
    }
}

macro_rules! number_to_prim {
    ($($t:ty),* $(,)?) => {$(
        impl TryFrom<Number> for $t {
            type Error = Error;

            fn try_from(n: Number) -> Result<Self, Self::Error> {
                match n {
                    Number::Integer(i) => <$t>::try_from(i),
                    Number::Float(f)   => <$t>::try_from(f),
                }
            }
        }
    )*};
}
number_to_prim!(i8, i16, i32, i64, u8, u16, u32, u64, f32);

impl TryFrom<Number> for f64 {
    type Error = Error;

    fn try_from(n: Number) -> Result<Self, Error> {
        match n {
            Number::Integer(i) => f64::try_from(i),
            Number::Float(f) => Ok(f64::from(f)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_lossy<T: fmt::Debug>(r: Result<T, Error>) {
        match r {
            Err(Error::LossyConversion { .. }) => {}
            other => panic!("expected LossyConversion, got {other:?}"),
        }
    }

    #[test]
    fn lossy_error_carries_context() {
        let err = i8::try_from(SignedInteger::LInt(300)).unwrap_err();
        match err {
            Error::LossyConversion { from, to } => {
                assert_eq!(from, "300");
                assert_eq!(to, "i8");
            }
            other => panic!("expected LossyConversion, got {other:?}"),
        }
    }

    #[test]
    fn signed_to_int_lossless_widening() {
        assert_eq!(i32::try_from(SignedInteger::SInt(-7)).unwrap(), -7);
        assert_eq!(i32::try_from(SignedInteger::Int(-300)).unwrap(), -300);
        assert_eq!(i64::try_from(SignedInteger::DInt(-1)).unwrap(), -1);
        assert_eq!(u8::try_from(SignedInteger::SInt(127)).unwrap(), 127);
        assert_eq!(u64::try_from(SignedInteger::Int(32_767)).unwrap(), 32_767);
    }

    #[test]
    fn signed_to_int_rejects_overflow() {
        assert_lossy(i8::try_from(SignedInteger::Int(128)));
        assert_lossy(i8::try_from(SignedInteger::Int(-129)));
        assert_lossy(i32::try_from(SignedInteger::LInt(i64::MAX)));
        assert_lossy(i32::try_from(SignedInteger::LInt(i64::MIN)));
    }

    #[test]
    fn signed_to_unsigned_rejects_negative() {
        assert_lossy(u8::try_from(SignedInteger::SInt(-1)));
        assert_lossy(u64::try_from(SignedInteger::LInt(-1)));
    }

    #[test]
    fn signed_to_int_boundaries() {
        assert_eq!(i8::try_from(SignedInteger::SInt(i8::MAX)).unwrap(), i8::MAX);
        assert_eq!(i8::try_from(SignedInteger::SInt(i8::MIN)).unwrap(), i8::MIN);
        assert_lossy(i8::try_from(SignedInteger::Int(i8::MAX as i16 + 1)));
        assert_lossy(i8::try_from(SignedInteger::Int(i8::MIN as i16 - 1)));
        assert_eq!(
            u8::try_from(SignedInteger::Int(u8::MAX as i16)).unwrap(),
            u8::MAX
        );
        assert_lossy(u8::try_from(SignedInteger::Int(u8::MAX as i16 + 1)));
    }

    #[test]
    fn unsigned_to_int_lossless_widening() {
        assert_eq!(u32::try_from(UnsignedInteger::Byte(255)).unwrap(), 255);
        assert_eq!(
            u32::try_from(UnsignedInteger::UInt(65_535)).unwrap(),
            65_535
        );
        assert_eq!(
            u64::try_from(UnsignedInteger::UDInt(u32::MAX)).unwrap(),
            u32::MAX as u64
        );
        assert_eq!(i64::try_from(UnsignedInteger::Byte(0)).unwrap(), 0);
    }

    #[test]
    fn unsigned_to_signed_rejects_overflow() {
        assert_lossy(i8::try_from(UnsignedInteger::Byte(128)));
        assert_lossy(i32::try_from(UnsignedInteger::UDInt(u32::MAX)));
        assert_lossy(i64::try_from(UnsignedInteger::ULInt(u64::MAX)));
    }

    #[test]
    fn unsigned_to_int_boundaries() {
        assert_eq!(
            u8::try_from(UnsignedInteger::Byte(u8::MAX)).unwrap(),
            u8::MAX
        );
        assert_lossy(u8::try_from(UnsignedInteger::UInt(256)));
        assert_eq!(
            i64::try_from(UnsignedInteger::ULInt(i64::MAX as u64)).unwrap(),
            i64::MAX
        );
        assert_lossy(i64::try_from(UnsignedInteger::ULInt(i64::MAX as u64 + 1)));
    }

    #[test]
    fn signed_to_float_small_values_exact() {
        assert_eq!(f64::try_from(SignedInteger::SInt(-1)).unwrap(), -1.0);
        assert_eq!(
            f32::try_from(SignedInteger::DInt(123_456)).unwrap(),
            123_456.0
        );
    }

    #[test]
    fn signed_to_f64_precision_boundary() {
        let two53: i64 = 1 << 53;
        assert_eq!(
            f64::try_from(SignedInteger::LInt(two53)).unwrap(),
            two53 as f64
        );
        assert_lossy(f64::try_from(SignedInteger::LInt(two53 + 1)));
        assert_lossy(f64::try_from(SignedInteger::LInt(-(two53 + 1))));
    }

    #[test]
    fn signed_to_f64_extremes() {
        assert_lossy(f64::try_from(SignedInteger::LInt(i64::MAX)));
        assert_eq!(
            f64::try_from(SignedInteger::LInt(i64::MIN)).unwrap(),
            i64::MIN as f64
        );
    }

    #[test]
    fn signed_to_f32_precision_boundary() {
        let two24: i32 = 1 << 24;
        assert_eq!(
            f32::try_from(SignedInteger::DInt(two24)).unwrap(),
            two24 as f32
        );
        assert_lossy(f32::try_from(SignedInteger::DInt(two24 + 1)));
    }

    #[test]
    fn unsigned_to_f64_precision_boundary() {
        let two53: u64 = 1 << 53;
        assert_eq!(
            f64::try_from(UnsignedInteger::ULInt(two53)).unwrap(),
            two53 as f64
        );
        assert_lossy(f64::try_from(UnsignedInteger::ULInt(two53 + 1)));
    }

    #[test]
    fn unsigned_to_f64_extremes() {
        // u64::MAX = 2^64 - 1; as f64 it rounds *up* to 2^64, so it's lossy.
        assert_lossy(f64::try_from(UnsignedInteger::ULInt(u64::MAX)));
    }

    #[test]
    fn float_to_int_exact_integers() {
        assert_eq!(i32::try_from(Float::Real(42.0)).unwrap(), 42);
        assert_eq!(i64::try_from(Float::LReal(-1.0)).unwrap(), -1);
        assert_eq!(u8::try_from(Float::LReal(0.0)).unwrap(), 0);
    }

    #[test]
    fn float_to_int_rejects_fractional() {
        assert_lossy(i32::try_from(Float::Real(1.5)));
        assert_lossy(i32::try_from(Float::LReal(-0.5)));
    }

    #[test]
    fn float_to_int_rejects_non_finite() {
        assert_lossy(i32::try_from(Float::LReal(f64::NAN)));
        assert_lossy(i32::try_from(Float::LReal(f64::INFINITY)));
        assert_lossy(i32::try_from(Float::LReal(f64::NEG_INFINITY)));
        assert_lossy(i32::try_from(Float::Real(f32::NAN)));
    }

    #[test]
    fn float_to_int_rejects_out_of_range() {
        assert_lossy(u8::try_from(Float::LReal(-1.0)));
        assert_lossy(i8::try_from(Float::LReal(128.0)));
        assert_lossy(i8::try_from(Float::LReal(-129.0)));
        assert_lossy(i64::try_from(Float::LReal(i64::MAX as f64)));
        assert_lossy(u64::try_from(Float::LReal(u64::MAX as f64)));
    }

    #[test]
    fn float_to_int_boundaries() {
        assert_eq!(i8::try_from(Float::LReal(i8::MAX as f64)).unwrap(), i8::MAX);
        assert_eq!(i8::try_from(Float::LReal(i8::MIN as f64)).unwrap(), i8::MIN);
        assert_eq!(u8::try_from(Float::LReal(u8::MAX as f64)).unwrap(), u8::MAX);
    }

    #[test]
    fn real_to_f32_is_infallible() {
        assert_eq!(f32::try_from(Float::Real(1.5)).unwrap(), 1.5);
        assert_eq!(f32::try_from(Float::Real(-0.0)).unwrap(), -0.0);
    }

    #[test]
    fn lreal_to_f32_lossless() {
        assert_eq!(f32::try_from(Float::LReal(0.5)).unwrap(), 0.5);
        assert_eq!(f32::try_from(Float::LReal(-128.0)).unwrap(), -128.0);
    }

    #[test]
    fn lreal_to_f32_lossy() {
        assert_lossy(f32::try_from(Float::LReal(0.1)));
        assert_lossy(f32::try_from(Float::LReal(f64::MAX)));
    }

    #[test]
    fn lreal_to_f32_nan_is_accepted() {
        let r = f32::try_from(Float::LReal(f64::NAN)).unwrap();
        assert!(r.is_nan());
    }

    #[test]
    fn integer_to_prim_dispatches() {
        assert_eq!(i32::try_from(Integer::from(-5i8)).unwrap(), -5);
        assert_eq!(i32::try_from(Integer::from(5u8)).unwrap(), 5);
        assert_eq!(i64::try_from(Integer::from(-100i32)).unwrap(), -100);
        assert_eq!(u64::try_from(Integer::from(100u32)).unwrap(), 100);
        assert_eq!(f64::try_from(Integer::from(42i32)).unwrap(), 42.0);
        assert_eq!(f32::try_from(Integer::from(42i32)).unwrap(), 42.0);
    }

    #[test]
    fn integer_to_prim_checked() {
        assert_lossy(i8::try_from(Integer::from(200u8)));
        assert_lossy(u8::try_from(Integer::from(-1i8)));
        assert_lossy(i64::try_from(Integer::from(u64::MAX)));
        assert_lossy(u64::try_from(Integer::from(-1i8)));
    }

    #[test]
    fn integer_to_i64_u64_use_lossless_arms() {
        assert_eq!(i64::try_from(Integer::from(i64::MIN)).unwrap(), i64::MIN);
        assert_eq!(u64::try_from(Integer::from(u64::MAX)).unwrap(), u64::MAX);
    }

    #[test]
    fn number_to_prim_dispatches() {
        assert_eq!(i32::try_from(Number::from(-7i16)).unwrap(), -7);
        assert_eq!(f32::try_from(Number::from(2.5f32)).unwrap(), 2.5);
        assert_eq!(i64::try_from(Number::from(42u8)).unwrap(), 42);
        assert_eq!(u64::try_from(Number::from(42i8)).unwrap(), 42);
    }

    #[test]
    fn number_to_prim_checked() {
        assert_lossy(i32::try_from(Number::from(1.5f64)));
        assert_lossy(u8::try_from(Number::from(-1i8)));
        assert_lossy(i64::try_from(Number::from(u64::MAX)));
        assert_lossy(f32::try_from(Number::from(0.1f64)));
        assert_lossy(u64::try_from(Number::from(-1i64)));
    }

    #[test]
    fn number_to_f64_mixed_sources() {
        assert_eq!(f64::try_from(Number::from(1i32)).unwrap(), 1.0);
        assert_eq!(f64::try_from(Number::from(1.5f64)).unwrap(), 1.5);
        assert_eq!(f64::try_from(Number::from(1.5f32)).unwrap(), 1.5);
        assert_lossy(f64::try_from(Number::from(i64::MAX)));
    }

    #[test]
    fn number_to_f64_preserves_infinity_and_nan() {
        assert!(
            f64::try_from(Number::from(f64::INFINITY))
                .unwrap()
                .is_infinite()
        );
        assert!(f64::try_from(Number::from(f64::NAN)).unwrap().is_nan());
    }
}
