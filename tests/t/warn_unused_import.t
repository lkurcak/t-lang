mod m {
    pub fn f() -> int { 1 }

    pub fn g() -> int { 2 }
}

use m::f;
use m::{g, f as h};

fn main() {
    print(m::f() + g()); // expect: 3
}
// warning: unused import `f`
// warning: unused import `h`
