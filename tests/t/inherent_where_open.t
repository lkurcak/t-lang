// An inherent impl's bound on what isn't part of its type may be of any
// trait: it's proved where a method of it is called, and never in the
// search for a trait's impl, so it can't make that loop (see
// docs/29-closed-and-sealed-traits.md).
trait Show {
    fn show(&self) -> i64;
}

impl Show for i64 {
    fn show(&self) -> i64 { *self }
}

#[derive(Discard where I: Discard)]
struct W<I> { iter: I }

impl<I: Iterator> W<I> where I::Item: Show + Discard {
    fn total(&mut self) -> i64 with {I::e, panic} {
        let mut n = 0;
        loop {
            match self.iter.next() {
                Option::Some(x) => n = n + x.show(),
                Option::None => return n,
            }
        }
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&(W { iter: 0..4i64 }).total())?; // expect: 6
    Result::Ok(())
}
