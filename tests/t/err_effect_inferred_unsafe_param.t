// Nor through an effect parameter: `g` would be unsafe if its `apply` is.
#[allow(unused_effects)]
fn danger() -> i64 with unsafe { 1 }

fn apply<effect e, F: Fn() -> i64 with e + Discard>(f: F) -> i64 with e { f() }

fn g() -> i64 { apply(danger) }

fn main() {
    // SAFETY: `danger` has none.
    let _ = unsafe { g() };
}
// error: `apply` is unsafe, but it isn't in an `unsafe` block, and `g` doesn't say `with unsafe`
