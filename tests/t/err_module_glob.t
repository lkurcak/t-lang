mod a {
    pub fn f() -> i64 { 1 }
}

use a::*;

fn main() { print(f()); }
// error: glob imports
