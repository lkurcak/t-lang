// A module sees only its own items and what it imports, not those of the
// module around it.
fn helper() -> i64 { 1 }

mod m {
    pub fn f() -> i64 { helper() }
}

fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&m::f())?;  Result::Ok(()) }
// error: unknown variable `helper`
