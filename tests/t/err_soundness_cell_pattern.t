// A pattern can't reach inside a cell either.
use core::cell::Cell;

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let c = Cell::new(1i64);
    let Cell { value } = c;
    out.line(&value)?;
    Result::Ok(())
}
// error: private
