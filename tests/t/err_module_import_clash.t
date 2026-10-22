mod a {
    pub fn f() -> i64 { 1 }
}

use a::f;

fn f() -> i64 { 2 }

fn main() { print(f()); }
// error: `f` is already defined in this module
