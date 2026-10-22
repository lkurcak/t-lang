mod a {
    fn f() -> i64 { 1 }

    pub use self::f as g;
}

fn main() { print(a::g()); }
// error: `f` is private, so `pub use` can't make it public
