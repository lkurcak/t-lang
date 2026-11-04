mod line { pub fn width() -> u8 { 5 } }
mod game {
    use crate::line::width::{self};
}
fn main() {}
// error: `line` has no item `width`
