// `self` in a group of a `use` names the module the group is in.
mod line {
    #[derive(Copy, Discard)]
    pub enum Cell { Empty, Full }
    pub fn width() -> u8 { 5 }
}
mod game {
    use crate::line::{self, Cell};
    use crate::line::{self as l};
    pub fn f() -> u8 {
        let c = if l::width() > 9 { Cell::Empty } else { Cell::Full };
        match c {
            Cell::Full => line::width() + l::width(),
            line::Cell::Empty => 0,
        }
    }
}
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&game::f())?;
    Ok(())
}
// expect: 10
