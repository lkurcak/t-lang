// The closure returns an `i64`, not the `bool` the return type promises.
fn f() -> impl Fn(i64) -> bool + Discard { |x| x + 1 }

fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&f()(1))?;  Result::Ok(()) }
// error: expected `bool`, found `i64`
