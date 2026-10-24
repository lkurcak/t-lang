// A closure whose result isn't written gets no lifetime of its argument's:
// `|_x: &i64| r` returns a reference that lives as long as `r` does,
// whatever it's given. That's another type than `Fn(&i64) -> &i64`, which
// returns one as long-lived as its argument, and the impl doesn't apply.
// If it did, `out` would point to `n` after it's gone.
trait Call {
    fn call<'a>(&self, y: &'a i64) -> &'a i64;
}

#[derive(Discard where P: Discard)]
struct Wrap<P> {
    p: P,
}

impl<P: Fn(&i64) -> &i64> Call for Wrap<P> {
    fn call<'a>(&self, y: &'a i64) -> &'a i64 {
        (self.p)(y)
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut stdout = os.stdout;
    let y = 1;
    let out;
    {
        let n: i64 = 2;
        let r = &n;
        let w = Wrap { p: |_x: &i64| r };
        out = w.call(&y);
    }
    stdout.line(&(*out))?;
    Result::Ok(())
}
// error: doesn't implement `Call`
