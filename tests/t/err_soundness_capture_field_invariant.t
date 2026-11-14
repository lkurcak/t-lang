// A struct whose fields' refinements relate them is captured whole when
// a closure changes one: were only `r.lo` lent to the closure, `r.hi`
// could be lowered under it, and the closure's `r.lo = 5` would break
// `hi >= lo`, checked against the `hi` it was made with.
#[derive(Discard)]
struct R { lo: u32, hi: { h: u32 | h >= lo } }

fn main() {
    let mut r = R { lo: 0, hi: 10 };
    let mut c = || r.lo = 5;
    r.hi = 3; // error: cannot assign to `r.hi` while `r` is mutably borrowed
    c();
}
