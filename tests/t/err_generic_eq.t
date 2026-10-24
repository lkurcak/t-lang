// `T` may be a function, so `==` on it needs a bound (traits, step 6).
fn same<T>(a: T, b: T) -> bool { a == b }
fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&same(1, 1))?;  Result::Ok(()) }
// error: cannot compare values of type `T` with `==`
