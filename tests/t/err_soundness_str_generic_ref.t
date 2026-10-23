// A type parameter never stands for `str`, even behind a reference in the
// signature: what the body does with a `T` isn't in it.
fn id<T>(x: &T) -> &T { x }

fn main() {
    id("hello");
}
// error: a type parameter can't stand for `str`: it's only ever behind a reference
