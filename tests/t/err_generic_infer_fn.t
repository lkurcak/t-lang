// `T` isn't in `f`'s signature, so nothing at the call says what it is.
fn f<T>(x: i64) -> i64 { x }
fn main() { print(f(1)); }
// error: cannot infer `T` for this use of `f`; nothing here says what it is, so write it, as in `f::<i64>(..)`
