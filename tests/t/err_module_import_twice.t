mod a {
    pub fn f() -> int { 1 }
}

mod b {
    pub fn f() -> int { 2 }
}

use a::f;
use b::f;

fn main() { print(f()); }
// error: `f` is imported twice
