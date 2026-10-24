mod a;
fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&a::hidden())?;  Result::Ok(()) } // error: `hidden` is private to `a`
