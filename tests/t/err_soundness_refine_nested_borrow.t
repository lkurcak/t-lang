// Nor can a `&mut` to a place inside such a field be kept.
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
    let r = &mut s.a.b;
    *r = 0;
    out.line(&s.n)?;
    Result::Ok(())
}
// error: can't borrow `s.a.b` as `&mut`: it's in `s.a`: the refinement of field `n` of `S` names it, which a write through the reference could break
