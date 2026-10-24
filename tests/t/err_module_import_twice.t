mod a {
    pub fn f() -> i64 { 1 }
}

mod b {
    pub fn f() -> i64 { 2 }
}

use a::f;
use b::f;

fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&f())?;  Result::Ok(()) }
// error: `f` is imported twice
