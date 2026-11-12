// The closure keeps `n()` of `d.x`, but assigns `d.y.n`; the loop lends
// `d.y` to `touch`, which keeps its `n()`. What the closure keeps is of
// `d.x` alone: at the head, `d.y.n()` isn't known.
#[derive(Copy, Discard)]
struct C { n: u8, m: u8 }

impl C {
    #[measure]
    fn n(&self) -> u8 { self.n }

    fn touch(&mut self) ensures self.n() == old(self.n()) { self.m = 0; }
}

#[derive(Copy, Discard)]
struct D { x: C, y: C }

fn f(d: &mut D) -> u8 with {} {
    d.y.n = 1;
    let mut i: u8 = 0;
    while i < 2 {
        let _q = 10 / d.y.n;
        let mut t = || {
            d.x.touch();
            d.y.n = 0;
        };
        t();
        d.y.touch();
        i = i + 1;
    }
    0
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut d = D { x: C { n: 1, m: 1 }, y: C { n: 1, m: 1 } };
    out.line(&f(&mut d))?;
    Ok(())
}
// error: `/` may panic, but `f` doesn't say `with panic`
