// The caller learns what `ensures` says of the value after the call, and
// nothing else: `old` is the value before, which `reset` doesn't keep.
#[derive(Copy, Discard)]
struct Counter { n: usize }

impl Counter {
    #[measure]
    fn get(&self) -> usize { self.n }

    fn reset(&mut self) ensures self.get() == 0 {
        self.n = 0;
    }
}

fn pos(n: { n: usize | n > 0 }) -> usize { n }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut c = Counter { n: 3 };
    if c.get() > 0 {
        c.reset();
        out.line(&pos(c.get()))?;
    }
    Result::Ok(())
}
// error: can't prove `n > 0` for argument `n` of `pos`
