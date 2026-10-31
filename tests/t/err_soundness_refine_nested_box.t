// A box is part of the value: writing in it changes what's measured.
use alloc::boxed::Box;

#[derive(Discard)]
struct Inner { b: usize }

impl Inner {
    #[measure]
    fn m(&self) -> usize { self.b }
}

#[derive(Discard)]
struct S { a: Box<Inner>, n: { n: usize | n <= a.m() } }

fn zero(r: &mut usize) { *r = 0; }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut s = S { a: Box::new(Inner { b: 5 }), n: 0 };
    if s.a.m() >= 3 {
        s.n = 3;
        zero(&mut s.a.b);
    }
    out.line(&s.n)?;
    Result::Ok(())
}
// error: can't prove `n <= a.m()` for field `n` of `S`, which names `a`
