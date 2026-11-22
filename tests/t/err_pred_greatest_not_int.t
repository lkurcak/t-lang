// `greatest::<T>()` in a predicate is of an integer type
// (docs/92-where-a-bound-holds.md).
use core::num::greatest;

fn f(n: u8) {
    let mut i = 0u8;
    while i < n invariant i <= greatest::<bool>() {
        i = i + 1;
    }
}

fn main() {}
// error: `bool` isn't an integer type, which `greatest` is of
