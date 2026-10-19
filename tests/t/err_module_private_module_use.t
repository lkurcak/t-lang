mod a {
    mod b {
        pub fn f() -> int { 1 }
    }

    pub fn g() -> int { b::f() }
}

use a::b::f;

fn main() { print(f() + a::g()); }
// error: module `b` is private to `a`
