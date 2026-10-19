mod a {
    pub fn f() -> int { 1 }
}

use a::f;

fn f() -> int { 2 }

fn main() { print(f()); }
// error: `f` is already defined in this module
