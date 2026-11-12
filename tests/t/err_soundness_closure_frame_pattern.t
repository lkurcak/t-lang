// The closure's body binds a field of what it captured by `&mut`, and
// writes through it: a change no call keeps anything of.
#[derive(Copy, Discard)]
struct C { n: u8, m: u8 }

impl C {
    #[measure]
    fn n(&self) -> u8 { self.n }

    fn touch(&mut self) ensures self.n() == old(self.n()) { self.m = 0; }
}

fn f(c: &mut C) -> u8 with {} {
    c.n = 4;
    let mut t = || {
        c.touch();
        let C { n, .. } = c;
        *n = 0;
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
