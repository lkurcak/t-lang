// `b` is unsafe only through `a`'s effect parameter, at a call of `a`
// made while `a`'s effects are still being worked out: only once the
// group's are does `b` have `unsafe`, which is an error then.
#[allow(unused_effects)]
fn danger() with unsafe {}

fn a<effect e>(g: fn() with e, n: u8) {
    if n == 0 { g() } else { b(n - 1) }
}

fn b(n: u8) { a(danger, n) }

fn main() {
    // SAFETY: `danger` has none.
    unsafe { b(1) }
}
// error: `b` is unsafe, but it isn't in an `unsafe` block, and `a` doesn't say `with unsafe`
