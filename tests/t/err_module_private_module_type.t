// Through a private module in the middle of a path, even to something
// `pub` in it.
mod a {
    mod b {
        #[derive(Discard)]
        pub struct P { pub n: int }
    }

    pub fn n() -> int { 1 }
}

fn f(p: a::b::P) -> int { p.n }

fn main() { print(a::n()); }
// error: module `b` is private to `a`
