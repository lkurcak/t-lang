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
//! They're written once, for every integer type, as `T: Int`. In code like
//! that the checker knows only what holds of all of them: that the least
//! is at most 0 and the greatest at least 127, and what `least` and
//! `greatest` say. Each but `checked_mul` uses the operator only where the
//! checker proves it can't panic (see docs/08-refinement-types.md and
//! docs/11-integers.md), so it needn't be trusted; `checked_mul` would need
//! it to reason about products of unknowns, which it doesn't.

use crate::option::Option;

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

/// The least value of integer type `T`: `least::<i8>()` is -128, and
/// `least::<u8>()` is 0. The checker knows it is.
#[intrinsic]
export fn least<T: Int>() -> T;

/// The greatest value of integer type `T`: `greatest::<u8>()` is 255.
/// The checker knows it is.
#[intrinsic]
export fn greatest<T: Int>() -> T;

/// `a + b`, or `None` if it doesn't fit in a `T`.
export fn checked_add<T: Int>(a: T, b: T) -> Option<T> {
    if b > 0 {
        if a <= greatest::<T>() - b { Option::Some(a + b) } else { Option::None }
    } else if a >= least::<T>() - b {
        Option::Some(a + b)
    } else {
        Option::None
    }
}

/// `a - b`, or `None` if it doesn't fit in a `T`.
export fn checked_sub<T: Int>(a: T, b: T) -> Option<T> {
    if b < 0 {
        if a <= greatest::<T>() + b { Option::Some(a - b) } else { Option::None }
    } else if a >= least::<T>() + b {
        Option::Some(a - b)
    } else {
        Option::None
    }
}

/// `a * b`, or `None` if it doesn't fit in a `T`.
#[intrinsic]
export fn checked_mul<T: Int>(a: T, b: T) -> Option<T>;

/// `a / b`, rounded towards zero, or `None` if `b` is zero or the result
/// doesn't fit in a `T`, as the least `i8` divided by -1 doesn't. That's
/// `b < 0 && b + 1 == 0` here, as -1 isn't a literal of every integer
/// type.
export fn checked_div<T: Int>(a: T, b: T) -> Option<T> {
    if b == 0 || (b < 0 && b + 1 == 0 && a == least::<T>()) {
        Option::None
    } else {
        Option::Some(a / b)
    }
}

/// `a % b`, which has the sign of `a`, or `None` where `checked_div` is.
export fn checked_rem<T: Int>(a: T, b: T) -> Option<T> {
    if b == 0 || (b < 0 && b + 1 == 0 && a == least::<T>()) {
        Option::None
    } else {
        Option::Some(a % b)
    }
}

/// `-a`, or `None` if it doesn't fit in a `T`: for the least value of a
/// signed type, and for all but 0 of an unsigned one.
export fn checked_neg<T: Int>(a: T) -> Option<T> {
    if a > 0 {
        if a + least::<T>() <= 0 { Option::Some(-a) } else { Option::None }
    } else if a + greatest::<T>() >= 0 {
        Option::Some(-a)
    } else {
        Option::None
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
export fn abs<T: Signed>(x: T) -> T with panic {
    if x < 0 { -x } else { x }
}

/// How far `x` is from 0, or `None` at the least value.
export fn checked_abs<T: Signed>(x: T) -> Option<T> {
    if x < 0 { checked_neg(x) } else { Option::Some(x) }
}

/// How far `x` is from 0, modulo 2^w: the least value is its own.
export fn wrapping_abs<T: Signed>(x: T) -> T {
    if x < 0 { wrapping_neg(x) } else { x }
}
