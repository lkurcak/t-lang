mod m {
    pub fn f() -> i64 { 1 }

    pub fn g() -> i64 { 2 }
}

use m::f;
use m::{g, f as h};

fn main() with panic {
    print(m::f() + g()); // expect: 3
}
// warning: unused import `f`
// warning: unused import `h`
