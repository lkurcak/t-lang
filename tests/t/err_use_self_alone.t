mod line { pub fn width() -> u8 { 5 } }
mod game {
    use crate::line::self;
    pub fn f() -> u8 { line::width() }
}
fn main() {}
// error: `self` can only start a path, or name the module of a `{..}` group of a `use`: `use crate::line::{self};`
