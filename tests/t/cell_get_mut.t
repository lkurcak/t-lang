// Through a `&mut`, what's in a cell can be lent like any value.
use core::cell::Cell;

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut c = Cell::new(5i64);
    let m = c.get_mut();
    *m = *m + 1;
    out.line(&c.get())?; // expect: 6
    Ok(())
}
