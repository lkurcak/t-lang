// A file module is hermetic, as an inline one is: `helper` is the crate
// root's, not in scope in `a.t` without `crate::` or `super::`.
mod a;
fn helper() -> i64 { 1 }
fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&a::f())?;  Result::Ok(()) }
