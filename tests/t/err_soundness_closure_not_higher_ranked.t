// A closure whose result isn't written gets no lifetime of its argument's:
// `|_x: &int| r` returns a reference that lives as long as `r` does,
// whatever it's given. That's another type than `Fn(&int) -> &int`, which
// returns one as long-lived as its argument, and the impl doesn't apply.
// If it did, `out` would point to `n` after it's gone.
trait Call {
    fn call<'a>(&self, y: &'a int) -> &'a int;
}

#[derive(Discard where P: Discard)]
struct Wrap<P> {
    p: P,
}

impl<P: Fn(&int) -> &int> Call for Wrap<P> {
    fn call<'a>(&self, y: &'a int) -> &'a int {
        (self.p)(y)
    }
}

fn main() {
    let y = 1;
    let out;
    {
        let n = 2;
        let r = &n;
        let w = Wrap { p: |_x: &int| r };
        out = w.call(&y);
    }
    print(*out);
}
// error: doesn't implement `Call`
