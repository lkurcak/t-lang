// Through a private module in the middle of a path, even to something
// `pub` in it.
mod a {
    mod b {
        #[derive(Discard)]
        pub struct P { pub n: i64 }
    }

    pub fn n() -> i64 { 1 }
}

fn f(p: a::b::P) -> i64 { p.n }

fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&a::n())?;  Result::Ok(()) }
// error: module `b` is private to `a`
