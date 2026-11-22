// `old(n)` is what `n` was at the call, not what a `mut` parameter is by
// the loop: or an invariant could count on a value that's gone
// (docs/92-where-a-bound-holds.md).
fn g(mut n: u8) -> u8 with {} {
    let k = n;
    n = 200;
    let mut i = 0u8;
    while i < 1 invariant n == old(n) {
        i = i + 1;
    }
    k + (n - n)
}

fn main() with {} {
    let _b = g(1);
}
// error: can't prove `n == old(n)` for the loop's invariant, as the loop starts
