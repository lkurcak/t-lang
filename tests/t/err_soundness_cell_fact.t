// What a cell held is no fact about what it holds after a call that could
// change it through a `&`.
use core::cell::Cell;

fn reset(c: &Cell<i64>) { c.set(0); }

fn need(x: { x: i64 | x > 0 }) -> i64 { x }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let c = Cell::new(5);
    let before = c.get();
    if before > 0 {
        reset(&c);
        out.line(&need(c.get()))?;
    }
    Ok(())
}
// error: can't prove
