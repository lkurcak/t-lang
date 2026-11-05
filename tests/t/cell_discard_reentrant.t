// `set` puts the new value in before dropping the old one, so a `discard`
// that looks at the cell sees the new one.
use core::cell::Cell;

struct Noisy<'a> { n: i64, log: &'a Cell<i64> }

impl<'a> Discard for Noisy<'a> {
    fn discard(self) { self.log.set(self.n); }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let log = Cell::new(0);
    let c = Cell::new(Noisy { n: 1, log: &log });
    c.set(Noisy { n: 2, log: &log });
    out.line(&log.get())?; // expect: 1
    c.set(Noisy { n: 3, log: &log });
    out.line(&log.get())?; // expect: 2
    let last = c.into_inner();
    out.line(&last.n)?; // expect: 3
    Ok(())
}
