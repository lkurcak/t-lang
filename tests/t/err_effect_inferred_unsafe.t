// `unsafe` is never inferred: saying it is a promise to keep obligations
// the checker can't check, which a body can't make for its function.
#[allow(unused_effects)]
fn danger() with unsafe {}

fn g() { danger() }

fn main() {
    // SAFETY: `danger` has none.
    unsafe { g() }
}
// error: `danger` is unsafe, but it isn't in an `unsafe` block, and `g` doesn't say `with unsafe`
