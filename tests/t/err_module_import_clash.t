mod a {
    pub fn f() -> i64 { 1 }
}

use a::f;

fn f() -> i64 { 2 }

fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&f())?;  Result::Ok(()) }
// error: `f` is already defined in this module
