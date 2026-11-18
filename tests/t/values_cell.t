// What's in a cell isn't part of its value to the logic, which can't see
// it: it can change behind a shared reference. So a cell set through one
// is the same value, and nothing is learned of what's in it
// (docs/85-values-in-predicates.md).
use core::cell::Cell;

fn set(c: &mut Cell<u8>) ensures *c == old(*c) { c.set(5); }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut c = Cell::new(1);
    set(&mut c);
    out.line(&c.get())?; // expect: 5
    Ok(())
}
