// A write inside a field a refinement is about changes that field: `s.a.b`
// is part of `s.a`, whose measure `n` mustn't exceed.
#[derive(Copy, Discard)]
struct Inner { b: usize }

impl Inner {
    #[measure]
    fn m(&self) -> usize { self.b }
}

#[derive(Copy, Discard)]
struct S { a: Inner, n: { n: usize | n <= a.m() } }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut s = S { a: Inner { b: 5 }, n: 5 };
    s.a.b = 0;
    out.line(&s.n)?;
    Result::Ok(())
}
// error: can't prove `n <= a.m()` for field `n` of `S`, which names `a`
