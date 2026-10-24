fn inc(x: i64) -> i64 with panic { x + 1 }
fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&(inc::<i64>(1)))?;  Result::Ok(()) }
// error: `inc` takes no type arguments
