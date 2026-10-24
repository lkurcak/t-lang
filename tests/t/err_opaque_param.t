// `impl` is only for return types: a parameter takes a type parameter.
fn apply(f: impl Fn(i64) -> i64, x: i64) -> i64 { f(x) }

fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&apply(|x| x, 1))?;  Result::Ok(()) }
// error: `impl` types can only be written in a function's return type
