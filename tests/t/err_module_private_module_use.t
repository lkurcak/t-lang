mod a {
    mod b {
        pub fn f() -> i64 { 1 }
    }

    pub fn g() -> i64 { b::f() }
}

use a::b::f;

fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&(f() + a::g()))?;  Result::Ok(()) }
// error: module `b` is private to `a`
