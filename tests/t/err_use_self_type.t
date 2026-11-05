// `{self}` names a module; a type has a path of its own to import it by.
mod line {
    #[derive(Discard)]
    pub enum Cell { Empty, Full }
}
mod game {
    use crate::line::Cell::{self};
}
fn main() {}
// error: `Cell` isn't a module
