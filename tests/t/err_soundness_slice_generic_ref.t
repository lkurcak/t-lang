// Even one only ever behind a reference in the function's signature: what
// the body does with a `T` isn't in it.
fn id<T>(x: &T) -> &T { x }

fn main() {
    let s: &[int] = core::slice::empty();
    id(s);
}
// error: a type parameter can't stand for the slice `[int]`: it's only ever behind a reference
