// A cell is changed through a shared reference: here two, at once.
use core::cell::Cell;

fn bump(c: &Cell<i64>) with panic { c.set(c.get() + 1); }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let c = Cell::new(1);
    let a = &c;
    let b = &c;
    bump(a);
    bump(b);
    out.line(&a.get())?; // expect: 3
    let old = b.replace(10);
    out.line(&old)?; // expect: 3
    out.line(&c.into_inner())?; // expect: 10
    Ok(())
}
