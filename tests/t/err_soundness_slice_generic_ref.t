// Even one only ever behind a reference in the function's signature: what
// the body does with a `T` isn't in it.
fn id<T>(x: &T) -> &T { x }

fn main() {
    let s: &[i64] = core::slice::empty();
    id(s);
}
// error: a type parameter that isn't `unsized` can't stand for the slice `[i64]`, which is only ever behind a reference
