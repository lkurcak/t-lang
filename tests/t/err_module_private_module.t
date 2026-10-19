// `b` is private to `a`, so what's in it can't be reached from outside
// `a`, `pub` or not.
mod a {
    mod b {
        pub fn f() -> int { 1 }
    }

    pub fn g() -> int { b::f() }
}

fn main() { print(a::b::f()); }
// error: module `b` is private to `a`
