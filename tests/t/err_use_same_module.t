// `line` is a module here already; bringing it in again says so.
mod line { pub fn width() -> u8 { 5 } }
use crate::line;
fn main() {}
// error: `line` is already a name in this module
