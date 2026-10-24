mod a {
    pub fn f() -> i64 { 1 }
}

fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&a::nope())?;  Result::Ok(()) }
// error: `a` has no item `nope`
