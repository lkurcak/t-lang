// `T` isn't in `f`'s signature, so nothing at the call says what it is.
fn f<T>(x: i64) -> i64 { x }
fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&f(1))?;  Result::Ok(()) }
// error: cannot infer `T` for this use of `f`; nothing here says what it is, so write it, as in `f::<i64>(..)`
