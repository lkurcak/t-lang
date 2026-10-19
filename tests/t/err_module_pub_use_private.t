mod a {
    fn f() -> int { 1 }

    pub use self::f as g;
}

fn main() { print(a::g()); }
// error: `f` is private, so `pub use` can't make it public
