mod a {
    fn f() -> i64 { 1 }

    pub use self::f as g;
}

fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&a::g())?;  Result::Ok(()) }
// error: `f` is private, so `pub use` can't make it public
