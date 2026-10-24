// The error is in `a/b.t`, and says so.
mod a;
fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&a::b::f())?;  Result::Ok(()) }
