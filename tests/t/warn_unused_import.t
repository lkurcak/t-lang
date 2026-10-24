mod m {
    pub fn f() -> i64 { 1 }

    pub fn g() -> i64 { 2 }
}

use m::f;
use m::{g, f as h};

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&(m::f() + g()))?; // expect: 3
    Result::Ok(())
}
// warning: unused import `f`
// warning: unused import `h`
