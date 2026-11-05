// What's in a cell can't be reached by its field: a `&` to it would see it
// change.
use core::cell::Cell;

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let c = Cell::new(1i64);
    let r: &i64 = &c.value;
    c.set(2);
    out.line(r)?;
    Ok(())
}
// error: private
