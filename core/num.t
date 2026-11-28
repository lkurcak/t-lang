//! Integers, and arithmetic that can't panic.
//!
//! There are ten integer types: `i8`, `i16`, `i32`, `i64` and `isize`,
//! and `u8`, `u16`, `u32`, `u64` and `usize`. Each is the integers from
//! its least to its greatest, and the operators give the same number as in
//! the integers, ℤ, or panic when that doesn't fit in the type, and `/` and
//! `%` when dividing by zero: `a + b` is `checked_add(a, b)`, or a panic on
//! `None`. These say so instead, or wrap around, as a machine's arithmetic
//! does: modulo 2^w, for a type `w` bits wide.
//!
//! And two floating-point types, `f32` and `f64`, which are `Float`'s.
//!
//! They're written once, for every integer type, as `T: Int`. In code like
//! that the checker knows only what holds of all of them: that the least
//! is at most 0 and the greatest at least 127, and what `least` and
//! `greatest` say. Each uses the operator only where the checker proves it
//! can't panic (see docs/08-refinement-types.md, docs/11-integers.md and,
//! for `checked_mul`, docs/51-products.md), so it needn't be trusted.

use crate::cmp::Ordering;
use crate::fmt::{Display, Write};
use crate::option::Option;
use crate::result::Result;
use crate::str::Parse;

/// Says a type is one of the ten integer types, so the arithmetic
/// operators work on it, and literals like `0` and `127` are of it. It's
/// sealed: no impl can add to it, so code over a `T: Int` may count on
/// what all ten have in common, and an impl for every `T: Int` doesn't
/// overlap one for a type that isn't an integer type (see
/// docs/29-closed-and-sealed-traits.md).
#[intrinsic]
export sealed trait Int: Copy + Eq + Ord {}

/// Says an integer type is signed: its least value is -1 minus its
/// greatest (see docs/12-bits.md). Sealed too.
#[intrinsic]
export sealed trait Signed: Int {}

/// Says an integer type is unsigned: its least value is 0. Sealed too.
#[intrinsic]
export sealed trait Unsigned: Int {}

/// Says a type is one of the two floating-point types, `f32` and `f64`:
/// IEEE 754's binary32 and binary64. The arithmetic operators work on it,
/// each giving the exact result rounded to the nearest value of the type,
/// ties to even, and never panicking: what's too big is an infinity, and
/// what has no answer, like `0.0 / 0.0`, a NaN, "not a number". Literals
/// like `2.5` are of it, and `==` and `<` compare as IEEE 754 does: a NaN
/// is equal to nothing, itself included, and not less or greater than
/// anything, so a float isn't `Eq` or `Ord`, whose laws that breaks;
/// `total_cmp` orders every value. It's sealed: code over a `F: Float` may
/// count on what both have in common (see docs/101-floats.md).
///
/// A NaN in gives a NaN out, of every method too. Which NaN, its sign and
/// the rest of its bits, isn't said when arithmetic makes one.
export sealed trait Float: Copy {
    /// The value nearest integer `n`, ties to even: `f32::nearest(16777217)`
    /// is `16777216.0`, which `as` would refuse, as it's not that number.
    fn nearest<T: Int>(n: T) -> Self;

    /// The value nearest `x`, ties to even: `x` itself, of an `f64`.
    fn nearest_f64(x: f64) -> Self;

    /// Infinity, more than every other value but NaN.
    fn infinity() -> Self;

    /// A NaN, of no sign, the quiet one IEEE 754 makes by default.
    fn nan() -> Self;

    /// The greatest finite value: `3.4028235e38` of an `f32`.
    fn greatest() -> Self;

    /// The least positive value that has all of the type's precision, a
    /// normal one: `1.1754944e-38` of an `f32`. Others below it are
    /// subnormal.
    fn least_positive() -> Self;

    /// How far it is from 1 to the next value: 2^-23 of an `f32`, 2^-52 of
    /// an `f64`.
    fn epsilon() -> Self;

    /// Is its sign bit set? Of `-0.0` too, and of a NaN that has it.
    fn is_sign_negative(self) -> bool;

    /// Is it a NaN? It's the only value not equal to itself.
    fn is_nan(self) -> bool { self != self }

    /// Is it infinity or minus infinity?
    fn is_infinite(self) -> bool { !self.is_nan() && self - self != self - self }

    /// Is it neither infinite nor a NaN?
    fn is_finite(self) -> bool { self - self == self - self }

    /// It without its sign: its sign bit cleared, of a NaN too.
    fn abs(self) -> Self {
        if self.is_sign_negative() { -self } else { self }
    }

    /// It with `sign`'s sign: its magnitude, and `sign`'s sign bit.
    fn copysign(self, sign: Self) -> Self {
        if self.is_sign_negative() == sign.is_sign_negative() { self } else { -self }
    }

    /// The integer nearest it, toward zero: `trunc(-2.7)` is `-2.0`.
    fn trunc(self) -> Self;

    /// The greatest integer at most it: `floor(-2.5)` is `-3.0`.
    fn floor(self) -> Self {
        let t = self.trunc();
        if self < t { t - 1.0 } else { t }
    }

    /// The least integer at least it: `ceil(2.1)` is `3.0`, and `ceil(-0.5)`
    /// is `-0.0`.
    fn ceil(self) -> Self {
        let t = self.trunc();
        if self > t { t + 1.0 } else { t }
    }

    /// The integer nearest it, halfway ones away from zero: `round(2.5)` is
    /// `3.0`, and `round(-2.5)` is `-3.0`.
    fn round(self) -> Self {
        let a = self.abs();
        let t = a.trunc();
        // `a - t` is exact: `t` is within a factor of 2 of `a`, or 0.
        let r = if a - t >= 0.5 { t + 1.0 } else { t };
        r.copysign(self)
    }

    /// Its square root, rounded once: a NaN below zero, but of `-0.0`,
    /// which is `-0.0`.
    fn sqrt(self) -> Self;

    /// e^x. Each of the math functions below gives the float nearest the
    /// exact value, ties to even, for every input: the same bits on every
    /// machine, and of any correct implementation (docs/102-math.md). 1 of
    /// 0, infinity past about 709.78 (88.72 of an `f32`), 0 of minus
    /// infinity and below about -745.13 (-103.97).
    fn exp(self) -> Self;

    /// 2^x: exactly that power of two of an integer.
    fn exp2(self) -> Self;

    /// e^x - 1, precise where `x` is near 0, where `x.exp() - 1.0` isn't.
    fn exp_m1(self) -> Self;

    /// The natural logarithm, ln x: minus infinity of `±0.0`, a NaN below
    /// 0, 0 of 1.
    fn ln(self) -> Self;

    /// log2 x: exactly `k` of 2^k.
    fn log2(self) -> Self;

    /// log10 x: exactly `k` of 10^k.
    fn log10(self) -> Self;

    /// ln(1 + x), precise where `x` is near 0: minus infinity of -1, a NaN
    /// below it.
    fn ln_1p(self) -> Self;

    /// x^y, as IEEE 754's `pow`, which is C's: 1 of `y = ±0` and of `x =
    /// 1`, even where the other is a NaN; a NaN of a negative `x` and a `y`
    /// that isn't an integer; of a negative `x` and an integer `y`, negative
    /// where `y` is odd; exact where `x^y` is a float, and halfway ones to
    /// even.
    fn pow(self, y: Self) -> Self;

    /// The sine, of `x` in radians, however big: a NaN of an infinity.
    fn sin(self) -> Self;

    /// The cosine, of `x` in radians.
    fn cos(self) -> Self;

    /// The tangent, of `x` in radians: never infinite, as no float is a
    /// multiple of π/2.
    fn tan(self) -> Self;

    /// The hyperbolic sine.
    fn sinh(self) -> Self;

    /// The hyperbolic cosine.
    fn cosh(self) -> Self;

    /// The hyperbolic tangent: ±1 past about 19.1 (9.1 of an `f32`).
    fn tanh(self) -> Self;

    /// The arcsine, in radians, from -π/2 to π/2: a NaN past ±1.
    fn asin(self) -> Self;

    /// The arccosine, in radians, from 0 to π: a NaN past ±1.
    fn acos(self) -> Self;

    /// The arctangent, in radians, from -π/2 to π/2.
    fn atan(self) -> Self;

    /// The angle of the point `(x, y)`, of `y.atan2(x)`, in radians, from -π
    /// to π, its sign `y`'s: IEEE 754's (C's) where either is 0 or infinite,
    /// so `0.0.atan2(-0.0)` is π and `(-0.0).atan2(1.0)` is `-0.0`.
    fn atan2(self, x: Self) -> Self;

    /// The inverse hyperbolic sine.
    fn asinh(self) -> Self;

    /// The inverse hyperbolic cosine: a NaN below 1.
    fn acosh(self) -> Self;

    /// The inverse hyperbolic tangent: ±infinity of ±1, a NaN past them.
    fn atanh(self) -> Self;

    /// The cube root, of a negative number too: exact where it's a float.
    fn cbrt(self) -> Self;

    /// sqrt(x^2 + y^2), of `x.hypot(y)`, with no overflow or underflow on the
    /// way: exact where it's a float, and infinity if either is, even where
    /// the other is a NaN.
    fn hypot(self, y: Self) -> Self;

    /// sin(πx), worked out from `x` itself, so exact at every multiple of
    /// 1/2 and precise however big `x` is, which the sine of `x` times π isn't: 0 of
    /// an integer, with `x`'s sign.
    fn sin_pi(self) -> Self;

    /// cos(πx): ±1 of an integer, and 0 halfway between two.
    fn cos_pi(self) -> Self;

    /// The lesser of it and `other`: a NaN if either is, and `-0.0` of
    /// `0.0` and `-0.0` (IEEE 754-2019's `minimum`).
    fn min(self, other: Self) -> Self {
        if self.is_nan() {
            self
        } else if other.is_nan() || other < self {
            other
        } else if self < other || self.is_sign_negative() {
            self
        } else {
            other
        }
    }

    /// The greater of it and `other`: a NaN if either is, and `0.0` of
    /// `0.0` and `-0.0` (IEEE 754-2019's `maximum`).
    fn max(self, other: Self) -> Self {
        if self.is_nan() {
            self
        } else if other.is_nan() || other > self {
            other
        } else if self > other || !self.is_sign_negative() {
            self
        } else {
            other
        }
    }

    /// Where it comes in IEEE 754's total order of all values: negative
    /// NaNs, minus infinity, the negative numbers, `-0.0`, `0.0`, the
    /// positive numbers, infinity, positive NaNs; NaNs by their bits.
    fn total_cmp(&self, other: &Self) -> Ordering;
}

/// The least value of integer type `T`: `least::<i8>()` is -128, and
/// `least::<u8>()` is 0. The checker knows it is.
#[intrinsic]
export fn least<T: Int>() -> T;

/// The greatest value of integer type `T`: `greatest::<u8>()` is 255.
/// The checker knows it is.
#[intrinsic]
export fn greatest<T: Int>() -> T;

/// `a + b`, or `None` if it doesn't fit in a `T`: what's in the `Some` is
/// known to be `a + b`, so it can be used where that's needed.
export fn checked_add<T: Int>(a: T, b: T) -> Option<{ v: T | v == a + b }> {
    if b > 0 {
        if a <= greatest::<T>() - b { Some(a + b) } else { None }
    } else if a >= least::<T>() - b {
        Some(a + b)
    } else {
        None
    }
}

/// `a - b`, or `None` if it doesn't fit in a `T`.
export fn checked_sub<T: Int>(a: T, b: T) -> Option<{ v: T | v == a - b }> {
    if b < 0 {
        if a <= greatest::<T>() + b { Some(a - b) } else { None }
    } else if a >= least::<T>() + b {
        Some(a - b)
    } else {
        None
    }
}

/// `a * b`, or `None` if it doesn't fit in a `T`. It fits when `a` is no
/// further from 0 than the greatest (or least) `T` divided by `b`: what
/// `/` gives, the checker knows, is within one `b` of what's divided, on
/// the side of zero (docs/51-products.md).
export fn checked_mul<T: Int>(a: T, b: T) -> Option<{ v: T | v == a * b }> {
    if a == 0 || b == 0 {
        Some(0)
    } else if a > 0 {
        if b > 0 {
            if a <= greatest::<T>() / b { Some(a * b) } else { None }
        } else if b >= least::<T>() / a {
            Some(a * b)
        } else {
            None
        }
    } else if b > 0 {
        if a >= least::<T>() / b { Some(a * b) } else { None }
    } else if a >= greatest::<T>() / b {
        Some(a * b)
    } else {
        None
    }
}

/// `a / b`, rounded towards zero, or `None` if `b` is zero or the result
/// doesn't fit in a `T`, as the least `i8` divided by -1 doesn't. That's
/// `b < 0 && b + 1 == 0` here, as -1 isn't a literal of every integer
/// type.
export fn checked_div<T: Int>(a: T, b: T) -> Option<T> {
    if b == 0 || (b < 0 && b + 1 == 0 && a == least::<T>()) { None } else { Some(a / b) }
}

/// `a % b`, which has the sign of `a`, or `None` where `checked_div` is.
export fn checked_rem<T: Int>(a: T, b: T) -> Option<T> {
    if b == 0 || (b < 0 && b + 1 == 0 && a == least::<T>()) { None } else { Some(a % b) }
}

/// `-a`, or `None` if it doesn't fit in a `T`: for the least value of a
/// signed type, and for all but 0 of an unsigned one.
export fn checked_neg<T: Int>(a: T) -> Option<{ v: T | v == -a }> {
    if a > 0 {
        if a + least::<T>() <= 0 { Some(-a) } else { None }
    } else if a + greatest::<T>() >= 0 {
        Some(-a)
    } else {
        None
    }
}

/// `a + b`, modulo 2^w for a `T` that's `w` bits wide: past the greatest
/// value, it goes on from the least. `wrapping_add(255u8, 1)` is 0.
export fn wrapping_add<T: Int>(a: T, b: T) -> T {
    if b > 0 {
        if a <= greatest::<T>() - b {
            a + b
        } else {
            // a + b - 2^w, as 2^w is the greatest minus the least, plus 1.
            a - (greatest::<T>() - b) - 1 + least::<T>()
        }
    } else if a >= least::<T>() - b {
        a + b
    } else {
        // a + b + 2^w.
        a - (least::<T>() - b) + 1 + greatest::<T>()
    }
}

/// `a + b`, or the greatest value of `T` if it's more, or the least if it's
/// less: `saturating_add(250u8, 10)` is 255. It's at least `a` when `b`
/// isn't negative, and at most `a` when `b` isn't positive.
export fn saturating_add<T: Int>(a: T, b: T) -> { v: T | (b < 0 || v >= a) && (b > 0 || v <= a) } {
    if b > 0 {
        if a <= greatest::<T>() - b { a + b } else { greatest::<T>() }
    } else if a >= least::<T>() - b {
        a + b
    } else {
        least::<T>()
    }
}

/// `a - b`, or the greatest value of `T` if it's more, or the least if it's
/// less: `saturating_sub(3u8, 4)` is 0.
export fn saturating_sub<T: Int>(a: T, b: T) -> { v: T | (b > 0 || v >= a) && (b < 0 || v <= a) } {
    if b < 0 {
        if a <= greatest::<T>() + b { a - b } else { greatest::<T>() }
    } else if a >= least::<T>() + b {
        a - b
    } else {
        least::<T>()
    }
}

/// `-a`, modulo 2^w: `wrapping_neg(1u8)` is 255, and the least value of a
/// signed type is its own negation.
export fn wrapping_neg<T: Int>(a: T) -> T {
    if a > 0 {
        if a + least::<T>() <= 0 {
            -a
        } else {
            // -a + 2^w.
            greatest::<T>() - a + 1 + least::<T>()
        }
    } else if a + greatest::<T>() >= 0 {
        -a
    } else {
        // Only the least value of a signed type, -2^(w-1), gets here: and
        // -a - 2^w is that.
        a
    }
}

/// `a - b`, modulo 2^w: `wrapping_sub(0u8, 1)` is 255.
export fn wrapping_sub<T: Int>(a: T, b: T) -> T { wrapping_add(a, wrapping_neg(b)) }

/// `a * b`, modulo 2^w.
#[intrinsic]
export fn wrapping_mul<T: Int>(a: T, b: T) -> T;

/// `x` modulo 2^w, for a `U` that's `w` bits wide, in `U`'s range: the
/// bits of `x`'s two's complement that fit in a `U`, as a `U`. So
/// `wrap::<i8, u64>(-1)` is the greatest `u64`, `wrap::<u16, u8>(300)` is
/// 44, and `wrap::<u8, i8>(200)` is -56. Where `x` fits in a `U`, it's
/// `x`, as `x as U` is: but this is for every pair of integer types, so
/// generic code can use it, as to hash any integer (see docs/12-bits.md).
#[intrinsic]
export fn wrap<T: Int, U: Int>(x: T) -> U;

/// How far `x` is from 0: it panics at the least value, whose distance is
/// one more than the greatest.
#[blame_caller]
export fn abs<T: Signed>(x: T) -> T with panic {
    if x < 0 { -x } else { x }
}

/// How far `x` is from 0, or `None` at the least value.
export fn checked_abs<T: Signed>(x: T) -> Option<{ v: T | v >= 0 && (v == x || v == -x) }> {
    if x < 0 { checked_neg(x) } else { Some(x) }
}

/// How far `x` is from 0, modulo 2^w: the least value is its own.
export fn wrapping_abs<T: Signed>(x: T) -> T {
    if x < 0 { wrapping_neg(x) } else { x }
}

/// Decimal, with a `+` or a `-` first if it's wanted, and nothing else:
/// `"42"`, `"-7"`, `"+007"`. A `-` before a number of an unsigned type is
/// an invalid digit, even before `0`.
impl<T: Int> Parse for T {
    type Error = ParseIntError;

    fn parse(s: &str) -> Result<T, ParseIntError> {
        let b = s.as_bytes();
        if b.len() == 0 {
            return Err(ParseIntError::Empty);
        }
        let negative = b[0] == 0x2D;
        if negative && least::<T>() == 0 {
            return Err(ParseIntError::InvalidDigit);
        }
        let mut i: usize = if negative || b[0] == 0x2B { 1 } else { 0 };
        if i == b.len() {
            return Err(ParseIntError::InvalidDigit);
        }
        let overflow = if negative {
            ParseIntError::NegOverflow
        } else {
            ParseIntError::PosOverflow
        };
        let mut n: T = 0;
        while i < b.len() {
            let c = b[i];
            if c < 0x30 || c > 0x39 {
                return Err(ParseIntError::InvalidDigit);
            }
            let d: T = wrap(c - 0x30);
            let Some(m) = checked_mul(n, 10) else { return Err(overflow) };
            let next = if negative { checked_sub(m, d) } else { checked_add(m, d) };
            let Some(next) = next else { return Err(overflow) };
            n = next;
            i = i + 1;
        }
        Ok(n)
    }
}

/// What's wrong with a string that isn't an integer of a type.
#[derive(Copy, Debug, Discard, Eq, Send, Sync)]
export enum ParseIntError {
    /// It has no characters.
    Empty,
    /// One isn't a digit, or a sign where one can be.
    InvalidDigit,
    /// It's greater than the type's greatest.
    PosOverflow,
    /// It's less than the type's least.
    NegOverflow,
}

impl Display for ParseIntError {
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e {
        w.write_str(match self {
            ParseIntError::Empty => "cannot parse integer from empty string",
            ParseIntError::InvalidDigit => "invalid digit found in string",
            ParseIntError::PosOverflow => "number too large to fit in target type",
            ParseIntError::NegOverflow => "number too small to fit in target type",
        })
    }
}
