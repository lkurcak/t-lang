mod a {
    pub fn f() -> i64 { 1 }
}

use a::*;

fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&f())?;  Result::Ok(()) }
// error: glob imports
