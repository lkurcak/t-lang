// Integers, and arithmetic that can't panic.
//
// There are ten integer types: `i8`, `i16`, `i32`, `i64` and `isize`,
// and `u8`, `u16`, `u32`, `u64` and `usize`. Each is the integers from
// its least to its greatest, and the operators give the same number as in
// the integers, ℤ, or panic when that doesn't fit in the type, and `/` and
// `%` when dividing by zero: `a + b` is `checked_add(a, b)`, or a panic on
// `None`. These say so instead.
//
// They're written once, for every integer type, as `T: Int`. In code like
// that the checker knows only what holds of all of them: that the least
// is at most 0 and the greatest at least 127, and what `least` and
// `greatest` say. Each but `checked_mul` uses the operator only where the
// checker proves it can't panic (see docs/08-refinement-types.md and
// docs/11-integers.md), so it needn't be trusted; `checked_mul` would need
// it to reason about products of unknowns, which it doesn't.

use crate::option::Option;

/// The least value of integer type `T`: `least::<i8>()` is -128, and
/// `least::<u8>()` is 0. The checker knows it is.
#[intrinsic]
pub fn least<T: Int>() -> T;

/// The greatest value of integer type `T`: `greatest::<u8>()` is 255.
/// The checker knows it is.
#[intrinsic]
pub fn greatest<T: Int>() -> T;

/// `a + b`, or `None` if it doesn't fit in a `T`.
pub fn checked_add<T: Int>(a: T, b: T) -> Option<T> {
    if b > 0 {
        if a <= greatest::<T>() - b { Option::Some(a + b) } else { Option::None }
    } else if a >= least::<T>() - b {
        Option::Some(a + b)
    } else {
        Option::None
    }
}

/// `a - b`, or `None` if it doesn't fit in a `T`.
pub fn checked_sub<T: Int>(a: T, b: T) -> Option<T> {
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
pub fn checked_mul<T: Int>(a: T, b: T) -> Option<T>;

/// `a / b`, rounded towards zero, or `None` if `b` is zero or the result
/// doesn't fit in a `T`, as the least `i8` divided by -1 doesn't. That's
/// `b < 0 && b + 1 == 0` here, as -1 isn't a literal of every integer
/// type.
pub fn checked_div<T: Int>(a: T, b: T) -> Option<T> {
    if b == 0 || (b < 0 && b + 1 == 0 && a == least::<T>()) {
        Option::None
    } else {
        Option::Some(a / b)
    }
}

/// `a % b`, which has the sign of `a`, or `None` where `checked_div` is.
pub fn checked_rem<T: Int>(a: T, b: T) -> Option<T> {
    if b == 0 || (b < 0 && b + 1 == 0 && a == least::<T>()) {
        Option::None
    } else {
        Option::Some(a % b)
    }
}

/// `-a`, or `None` if it doesn't fit in a `T`: for the least value of a
/// signed type, and for all but 0 of an unsigned one.
pub fn checked_neg<T: Int>(a: T) -> Option<T> {
    if a > 0 {
        if a + least::<T>() <= 0 { Option::Some(-a) } else { Option::None }
    } else if a + greatest::<T>() >= 0 {
        Option::Some(-a)
    } else {
        Option::None
    }
}
