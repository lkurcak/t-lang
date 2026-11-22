// `old(..)` in an invariant is of the function's parameters, as they were
// when it was called: a local has no value then
// (docs/92-where-a-bound-holds.md).
fn f(n: u8) {
    let mut i = 0u8;
    while i < n invariant i <= old(i) {
        i = i + 1;
    }
}

fn main() {}
// error: `old(..)` in an invariant is of the function's parameters, as they were when it was called: `i` isn't one
