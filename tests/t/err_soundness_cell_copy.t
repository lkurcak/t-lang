// A cell isn't `Copy`: a copy would be a second cell, which a change to
// the first doesn't change.
use core::cell::Cell;

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let c = Cell::new(1i64);
    let d = c;
    c.set(2);
    out.line(&d.get())?;
    Ok(())
}
// error: moved
