// Inside `as_i64`, `T` could be any type, so it isn't an `i64`.
fn as_i64<T>(x: T) -> i64 { x }
fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&as_i64(1i64))?;  Result::Ok(()) }
// error: expected `i64`, found `T`
