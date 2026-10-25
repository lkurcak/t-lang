// An array of a type parameter's values may only be dropped if they may.
fn ignore<T, const N: usize>(a: [T; N]) {}

fn main() {}
// warning: unused variable `a`: use it up, as its type, `[T; N]`, isn't `Discard`
// error: `a` is never used up, and its type, `[T; N]`, isn't `Discard`
