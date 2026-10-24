#[derive(Discard)]
struct P { x: i64 }
fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&(P { x: 1 } == P { x: 1 }))?;  Result::Ok(()) }
// error: cannot compare `P` with `==`: it doesn't implement `Eq`; add `#[derive(Eq)]` to it
