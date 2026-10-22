mod a {
    mod b {
        pub fn f() -> i64 { 1 }
    }

    pub fn g() -> i64 { b::f() }
}

use a::b::f;

fn main() { print(f() + a::g()); }
// error: module `b` is private to `a`
