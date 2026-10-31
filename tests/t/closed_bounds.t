// A bound on what isn't a part of an impl's type may be of a closed trait:
// `Discard`, `Copy`, the `Fn` traits, `Int`, `Signed` and `Unsigned`.
// Proving one of those never gets to any other trait, so it can't lead
// back to the impl (see docs/29-closed-and-sealed-traits.md).
trait Count {
    effect e;

    fn positives(&mut self) -> i64 with {Self::e, panic};
}

#[derive(Discard where I: Discard)]
struct Positives<I> { iter: I }

impl<I: Iterator> Count for Positives<I> where I::Item: Int {
    effect e = I::e;

    fn positives(&mut self) -> i64 with {I::e, panic} {
        let mut n = 0;
        loop {
            match self.iter.next() {
                Option::Some(x) => n = n + if x > 0 { 1 } else { 0 },
                Option::None => return n,
            }
        }
    }
}

trait Sign {
    fn negative(&self) -> bool;
}

#[derive(Discard)]
struct Wrap<I> { first: I }

impl<U: Copy, I: Iterator<Item = U>> Sign for Wrap<I> where U: Signed {
    fn negative(&self) -> bool { false }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&(Positives { iter: -3..4i32 }).positives())?; // expect: 3
    out.line(&(Wrap { first: 0..1i8 }).negative())?;        // expect: false
    Result::Ok(())
}
