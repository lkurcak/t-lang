//! Floating-point numbers: `f32` and `f64`, IEEE 754's binary32 and
//! binary64, and what they are as `Float`s (see docs/101-floats.md).
//!
//! What the checker takes on trust here is each type's bits, `to_bits` and
//! `from_bits`, the square root, and the nearest float of an integer or an
//! `f64`: all exactly what IEEE 754 says. The rest is written in T, over
//! the operators, which are IEEE 754's too.

use crate::cmp::Ordering;
use crate::fmt::{Debug, Display, Write};
use crate::num::{Float, Int, wrap};
use crate::result::Result;
use crate::str::Parse;

mod dec;

export use self::dec::ParseFloatError;

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
fn trunc_below<F: Float>(x: F, big: F) -> F with {} {
    let a = x.abs();
    if a < big {
        let r = a + big - big;
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
fn order_key(b: u64, sign: u64) -> u64 with {} {
    if b >= sign { !b } else { b | 0x8000_0000_0000_0000 }
}

/// `a` against `b`, as unsigned keys.
fn by_keys(a: u64, b: u64) -> Ordering with {} {
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
        by_keys(order_key(self.to_bits() as u64, sign), order_key(other.to_bits() as u64, sign))
    }
}

/// The `f64` that `parts` are of: `parse` makes them so that its
/// significand fits, with its leading 1 at bit 52, or below for a
/// subnormal, whose exponent is the least.
fn of_parts_f64(parts: dec::Parts) -> f64 with {} {
    match parts {
        dec::Parts::NaN => f64::nan(),
        dec::Parts::Infinite(negative) => {
            if negative { -f64::infinity() } else { f64::infinity() }
        },
        dec::Parts::Finite(negative, m, e) => {
            let sign: u64 = if negative { 0x8000_0000_0000_0000 } else { 0 };
            let bits = if m < 0x0010_0000_0000_0000 {
                m & 0x000F_FFFF_FFFF_FFFF
            } else {
                let exp: u64 = if e > -1075 && e < 972 { (e + 1075) as u64 } else { 0 };
                ((exp & 0x7FF) << 52) | (m & 0x000F_FFFF_FFFF_FFFF)
            };
            f64::from_bits(sign | bits)
        },
    }
}

/// The `f32` that `parts` are of, as `of_parts_f64`.
fn of_parts_f32(parts: dec::Parts) -> f32 with {} {
    match parts {
        dec::Parts::NaN => f32::nan(),
        dec::Parts::Infinite(negative) => {
            if negative { -f32::infinity() } else { f32::infinity() }
        },
        dec::Parts::Finite(negative, m, e) => {
            let sign: u32 = if negative { 0x8000_0000 } else { 0 };
            let m32: u32 = wrap(m);
            let bits = if m < 0x0080_0000 {
                m32 & 0x007F_FFFF
            } else {
                let exp: u32 = if e > -150 && e < 105 { (e + 150) as u32 } else { 0 };
                ((exp & 0xFF) << 23) | (m32 & 0x007F_FFFF)
            };
            f32::from_bits(sign | bits)
        },
    }
}

/// Writes `x` with `places` digits after its point: `{x:.3}` (see
/// `fmt::Fixed`).
#[blame_caller]
pub fn write_fixed<W: Write>(x: f64, places: usize, w: &mut W) -> Result<(), W::Error> with W::e {
    dec::write_fixed(dec::parts_f64(x), places, w)
}

/// The fewest digits that read back as it, and of those the nearest: `0.1`,
/// `1.0`, `-0.0`, `1e16`, `1.5e-7`, `inf`, `NaN`. In decimal, with at least
/// one digit after the point, from `0.0001` up to, not including, `1e16`;
/// else in scientific notation. As Rust's `{:?}` writes it.
impl Display for f64 {
    #[blame_caller]
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e {
        dec::write_shortest(dec::parts_f64(*self), dec::f64_format(), w)
    }
}

/// As an `f64` is written, with an `f32`'s digits: `0.1f32` is `0.1`.
impl Display for f32 {
    #[blame_caller]
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e {
        dec::write_shortest(dec::parts_f32(*self), dec::f32_format(), w)
    }
}

/// As its `Display`.
impl Debug for f64 {
    #[blame_caller]
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { Display::fmt(self, w) }
}

/// As its `Display`.
impl Debug for f32 {
    #[blame_caller]
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { Display::fmt(self, w) }
}

/// The `f64` nearest the decimal, ties to even: digits, with a `.` and
/// more digits or not, or a `.` and digits, then an exponent or not, `e`
/// or `E` and digits, with a sign or not; or `inf`, `infinity` or `nan`, in
/// either case; with a `+` or a `-` first, or not. What `Display` writes
/// reads back as the same bits, but for a NaN's.
impl Parse for f64 {
    type Error = ParseFloatError;

    fn parse(s: &str) -> Result<f64, ParseFloatError> {
        match dec::parse(s.as_bytes(), dec::f64_format()) {
            Ok(parts) => Ok(of_parts_f64(parts)),
            Err(e) => Err(e),
        }
    }
}

/// The `f32` nearest the decimal, as for an `f64`: rounded once, from the
/// decimal, never through an `f64`.
impl Parse for f32 {
    type Error = ParseFloatError;

    fn parse(s: &str) -> Result<f32, ParseFloatError> {
        match dec::parse(s.as_bytes(), dec::f32_format()) {
            Ok(parts) => Ok(of_parts_f32(parts)),
            Err(e) => Err(e),
        }
    }
}
