//! Floating-point numbers: `f32` and `f64`, IEEE 754's binary32 and
//! binary64, and what they are as `Float`s (see docs/101-floats.md).
//!
//! What the checker takes on trust here is each type's bits, `to_bits` and
//! `from_bits`, the square root, and the nearest float of an integer or an
//! `f64`: all exactly what IEEE 754 says. The rest is written in T, over
//! the operators, which are IEEE 754's too.

use crate::cmp::Ordering;
use crate::num::{Float, Int};

/// The square root of `x`, rounded once.
#[intrinsic]
fn sqrt<F: Float>(x: F) -> F;

/// The `F` nearest integer `n`, ties to even.
#[intrinsic]
fn nearest<F: Float, T: Int>(n: T) -> F;

/// The `F` nearest `x`, ties to even.
#[intrinsic]
fn nearest_f64<F: Float>(x: f64) -> F;

/// `x` toward zero, to an integer, if it's less than `big` either way, the
/// least power of two whose neighbours are all integers: adding `big` to
/// its magnitude rounds to an integer, which taking `big` away again
/// leaves exactly, and that's one too many if it rounded up. Else `x` is
/// an integer already, an infinity or a NaN, and it's `x`.
fn trunc_below<F: Float>(x: F, big: F) -> F {
    let a = x.abs();
    if a < big {
        let r = (a + big) - big;
        let t = if r > a { r - 1.0 } else { r };
        t.copysign(x)
    } else {
        x
    }
}

/// The key `total_cmp` orders a float of bits `b` by, of a type whose sign
/// bit is `sign`: unsigned, ascending in IEEE 754's total order. A negative
/// one's bits, every one flipped, are less the less it is, and below every
/// positive one's, which get the top bit set.
fn order_key(b: u64, sign: u64) -> u64 {
    if b >= sign { !b } else { b | 0x8000_0000_0000_0000 }
}

/// `a` against `b`, as unsigned keys.
fn by_keys(a: u64, b: u64) -> Ordering {
    if a < b { Ordering::Less } else if a == b { Ordering::Equal } else { Ordering::Greater }
}

impl f64 {
    /// Its bits: the sign, then 11 of the exponent, then 52 of the
    /// significand, as IEEE 754 lays them out.
    #[intrinsic]
    export fn to_bits(self) -> u64;

    /// The `f64` of bits `b`: a NaN keeps every one of them.
    #[intrinsic]
    export fn from_bits(b: u64) -> f64;
}

impl f32 {
    /// Its bits: the sign, then 8 of the exponent, then 23 of the
    /// significand.
    #[intrinsic]
    export fn to_bits(self) -> u32;

    /// The `f32` of bits `b`.
    #[intrinsic]
    export fn from_bits(b: u32) -> f32;
}

impl Float for f64 {
    fn nearest<T: Int>(n: T) -> f64 { nearest(n) }

    fn nearest_f64(x: f64) -> f64 { x }

    fn infinity() -> f64 { f64::from_bits(0x7FF0_0000_0000_0000) }

    fn nan() -> f64 { f64::from_bits(0x7FF8_0000_0000_0000) }

    fn greatest() -> f64 { 1.7976931348623157e308 }

    fn least_positive() -> f64 { 2.2250738585072014e-308 }

    fn epsilon() -> f64 { 2.220446049250313e-16 }

    fn is_sign_negative(self) -> bool { self.to_bits() >= 0x8000_0000_0000_0000 }

    fn trunc(self) -> f64 { trunc_below(self, 4503599627370496.0) }

    fn sqrt(self) -> f64 { sqrt(self) }

    fn total_cmp(&self, other: &f64) -> Ordering {
        let sign = 0x8000_0000_0000_0000;
        by_keys(order_key(self.to_bits(), sign), order_key(other.to_bits(), sign))
    }
}

impl Float for f32 {
    fn nearest<T: Int>(n: T) -> f32 { nearest(n) }

    fn nearest_f64(x: f64) -> f32 { nearest_f64(x) }

    fn infinity() -> f32 { f32::from_bits(0x7F80_0000) }

    fn nan() -> f32 { f32::from_bits(0x7FC0_0000) }

    fn greatest() -> f32 { 3.4028235e38 }

    fn least_positive() -> f32 { 1.1754944e-38 }

    fn epsilon() -> f32 { 1.1920929e-7 }

    fn is_sign_negative(self) -> bool { self.to_bits() >= 0x8000_0000 }

    fn trunc(self) -> f32 { trunc_below(self, 8388608.0) }

    fn sqrt(self) -> f32 { sqrt(self) }

    fn total_cmp(&self, other: &f32) -> Ordering {
        let sign = 0x8000_0000;
        by_keys(
            order_key(self.to_bits() as u64, sign),
            order_key(other.to_bits() as u64, sign),
        )
    }
}
