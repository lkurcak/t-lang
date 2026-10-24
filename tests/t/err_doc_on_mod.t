// A module's doc comment goes in it, as `//!` lines at its start, as it
// does for a module in a file of its own. The fix moves it there.
/// Shapes.
mod shapes {
    pub fn one() -> i64 { 1 }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&shapes::one())?;
    Result::Ok(())
}
// error: a module's doc comment goes inside it, as `//!` lines at the start
