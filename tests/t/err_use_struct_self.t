// `{self}` in a group names a module or an enum: a struct has nothing a
// group could hold.
mod shapes {
    pub struct P { pub x: u8 }
}

use shapes::P::{self};

fn main() {}
// error: `P` isn't a module or an enum
