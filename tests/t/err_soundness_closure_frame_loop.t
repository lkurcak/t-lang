// A loop makes a closure that keeps `n()`, and also lends `c` to `zero`:
// the head doesn't know `n()`.
#[derive(Copy, Discard)]
struct C { n: u8, m: u8 }

impl C {
    #[measure]
    fn n(&self) -> u8 { self.n }

    fn touch(&mut self) ensures self.n() == old(self.n()) { self.m = 0; }
}

fn zero(c: &mut C) { c.n = 0; }

fn f(c: &mut C) -> u8 with {} {
    c.n = 4;
    let mut i: u8 = 0;
    while i < 3 {
        let _q = 8 / c.n;
        let mut t = || c.touch();
        t();
        zero(c);
        i = i + 1;
    }
    0
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut c = C { n: 1, m: 1 };
    out.line(&f(&mut c))?;
    Ok(())
}
// error: `/` may panic, but `f` doesn't say `with panic`
