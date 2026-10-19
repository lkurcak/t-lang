mod a {
    pub fn f() -> int { 1 }
}

use a::*;

fn main() { print(f()); }
// error: glob imports
