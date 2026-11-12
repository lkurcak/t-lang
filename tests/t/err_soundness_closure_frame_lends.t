// The closure lends `c` to `touch`, which keeps `n()`, and to `zero`,
// which says nothing: its loan keeps nothing.
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
    let mut t = || {
        c.touch();
        zero(c);
    };
    t();
    8 / c.n
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut c = C { n: 1, m: 1 };
    out.line(&f(&mut c))?;
    Ok(())
}
// error: `/` may panic, but `f` doesn't say `with panic`
