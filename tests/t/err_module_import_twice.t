mod a {
    pub fn f() -> i64 { 1 }
}

mod b {
    pub fn f() -> i64 { 2 }
}

use a::f;
use b::f;

fn main() { print(f()); }
// error: `f` is imported twice
