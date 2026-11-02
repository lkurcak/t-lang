// While what's in a cell is lent `&mut`, the cell can't be reached.
use core::cell::Cell;

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut c = Cell::new(1i64);
    let m = c.get_mut();
    c.set(2);
    *m = 3;
    out.line(&c.get())?;
    Result::Ok(())
}
// error: borrowed
