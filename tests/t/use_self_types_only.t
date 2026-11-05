// `{self}` brings in the module and only it: not a function of the same
// name beside it, so this module can have its own.
mod a {
    pub mod line { pub fn w() -> u8 { 1 } }
    pub fn line() -> u8 { 2 }
}
mod b {
    use crate::a::line::{self};
    fn line() -> u8 { 3 }
    pub fn f() -> u8 { line::w() + line() + crate::a::line() }
}
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&b::f())?;
    Ok(())
}
// expect: 6
