// Integers, and arithmetic that can't panic.
//
// The operators panic when the result doesn't fit in an `int`, and `/`
// and `%` when dividing by zero: `a + b` is `checked_add(a, b)`, or a
// panic on `None`. These say so instead.

use crate::option::Option;

/// `a + b`, or `None` if it doesn't fit in an `int`.
#[intrinsic]
pub fn checked_add(a: int, b: int) -> Option<int>;

/// `a - b`, or `None` if it doesn't fit in an `int`.
#[intrinsic]
pub fn checked_sub(a: int, b: int) -> Option<int>;

/// `a * b`, or `None` if it doesn't fit in an `int`.
#[intrinsic]
pub fn checked_mul(a: int, b: int) -> Option<int>;

/// `a / b`, rounded towards zero, or `None` if `b` is zero or the result
/// doesn't fit in an `int`, as the least `int` divided by `-1` doesn't.
#[intrinsic]
pub fn checked_div(a: int, b: int) -> Option<int>;

/// `a % b`, which has the sign of `a`, or `None` where `checked_div` is.
pub fn checked_rem(a: int, b: int) -> Option<int> {
    match checked_div(a, b) {
        Option::Some(q) => match checked_mul(q, b) {
            Option::Some(m) => checked_sub(a, m),
            Option::None => Option::None,
        },
        Option::None => Option::None,
    }
}

/// `-a`, or `None` for the least `int`, whose negation doesn't fit.
pub fn checked_neg(a: int) -> Option<int> { checked_sub(0, a) }
