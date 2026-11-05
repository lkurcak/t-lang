// A `self` in a group that nothing uses is an unused import like any other.
mod line {
    pub fn width() -> u8 { height() }
    pub fn height() -> u8 { 4 }
}
mod game {
    use crate::line::{self, height};
    pub fn f() -> u8 { height() }
    pub fn g() -> u8 { crate::line::width() }
}
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&(game::f() + game::g()))?;
    Result::Ok(())
}
// warning: unused import `line`
// expect: 8
