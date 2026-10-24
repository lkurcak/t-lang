// A function type holds functions, and closures that capture nothing: a
// closure that captures carries what it captures, so it isn't one. `n`
// lives in `adder`'s stack frame, which is gone once it returns.
fn adder(n: i64) -> fn(i64) -> i64 { |x| x + n }
fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&adder(5)(1))?;  Result::Ok(()) }
// error: expected `fn(i64) -> i64`, found a closure that captures `n`
