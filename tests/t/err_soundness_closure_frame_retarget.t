// The closure swaps `r` with `s`, so `r` points somewhere else: what `r` points to after isn't
// what it pointed to before, whatever's kept of that.
#[derive(Copy, Discard)]
struct C { n: u8, m: u8 }

impl C {
    #[measure]
    fn n(&self) -> u8 { self.n }

    fn touch(&mut self) ensures self.n() == old(self.n()) { self.m = 0; }
}

fn f<'a>(r: &'a mut C, s: &'a mut C) -> u8 with {} {
    r.n = 4;
    s.n = 0;
    let mut r = r;
    let mut s = s;
    let mut t = || {
        r.touch();
        core::mem::swap(&mut r, &mut s);
    };
    t();
    8 / r.n
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut a = C { n: 1, m: 1 };
    let mut b = C { n: 1, m: 1 };
    out.line(&f(&mut a, &mut b))?;
    Ok(())
}
// error: `/` may panic, but `f` doesn't say `with panic`
