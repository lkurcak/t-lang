// Integers, and arithmetic that can't panic.
//
// The operators panic when the result doesn't fit in an `int`, and `/`
// and `%` when dividing by zero: `a + b` is `checked_add(a, b)`, or a
// panic on `None`. These say so instead. Each but `checked_mul` uses
// the operator only where the checker proves it can't panic (see
// docs/08-refinement-types.md), so it needn't be trusted; `checked_mul`
// would need it to reason about products of unknowns, which it doesn't.

use crate::option::Option;

/// `a + b`, or `None` if it doesn't fit in an `int`.
pub fn checked_add(a: int, b: int) -> Option<int> {
    if b > 0 {
        if a <= 9223372036854775807 - b { Option::Some(a + b) } else { Option::None }
    } else if a >= -9223372036854775807 - 1 - b {
        Option::Some(a + b)
    } else {
        Option::None
    }
}

/// `a - b`, or `None` if it doesn't fit in an `int`.
pub fn checked_sub(a: int, b: int) -> Option<int> {
    if b < 0 {
        if a <= 9223372036854775807 + b { Option::Some(a - b) } else { Option::None }
    } else if a >= -9223372036854775807 - 1 + b {
        Option::Some(a - b)
    } else {
        Option::None
    }
}

/// `a * b`, or `None` if it doesn't fit in an `int`.
#[intrinsic]
pub fn checked_mul(a: int, b: int) -> Option<int>;

/// `a / b`, rounded towards zero, or `None` if `b` is zero or the result
/// doesn't fit in an `int`, as the least `int` divided by `-1` doesn't.
pub fn checked_div(a: int, b: int) -> Option<int> {
    if b == 0 || (a == -9223372036854775807 - 1 && b == -1) {
        Option::None
    } else {
        Option::Some(a / b)
    }
}

/// `a % b`, which has the sign of `a`, or `None` where `checked_div` is.
pub fn checked_rem(a: int, b: int) -> Option<int> {
    if b == 0 || (a == -9223372036854775807 - 1 && b == -1) {
        Option::None
    } else {
        Option::Some(a % b)
    }
}

/// `-a`, or `None` for the least `int`, whose negation doesn't fit.
pub fn checked_neg(a: int) -> Option<int> {
    if a == -9223372036854775807 - 1 { Option::None } else { Option::Some(-a) }
}
