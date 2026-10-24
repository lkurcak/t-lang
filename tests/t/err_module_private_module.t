// `b` is private to `a`, so what's in it can't be reached from outside
// `a`, `pub` or not.
mod a {
    mod b {
        pub fn f() -> i64 { 1 }
    }

    pub fn g() -> i64 { b::f() }
}

fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&a::b::f())?;  Result::Ok(()) }
// error: module `b` is private to `a`
