fn f() -> i64 { 1 }

fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&super::f())?;  Result::Ok(()) }
// error: `super` at the crate root
