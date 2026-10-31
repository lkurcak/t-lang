// Proving `W<X>: Tr` needs `X::Item: Tr2`, which is `V<W<X>>: Tr2`, which
// needs `W<X>: Tr` again: a bound on what isn't a part of an impl's type
// can lead back to the impl, and then finding out what implements what
// never ends. So such a bound can only be of a closed trait, whose proofs
// never get to any other (see docs/29-closed-and-sealed-traits.md).
trait Tr {
    fn a(&self) -> i64;
}

trait Tr2 {
    fn b(&self) -> i64;
}

#[derive(Discard where I: Discard)]
struct W<I> { iter: I }

#[derive(Discard where T: Discard)]
struct V<T> { t: T }

impl<I: Iterator> Tr for W<I> where I::Item: Tr2 {
    fn a(&self) -> i64 { 1 }
}

impl<T: Tr> Tr2 for V<T> {
    fn b(&self) -> i64 { 2 }
}

#[derive(Discard)]
struct X {}

impl Iterator for X {
    type Item = V<W<X>>;
    effect e = {};

    fn next(&mut self) -> Option<V<W<X>>> { Option::None }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&(W { iter: X {} }).a())?;
    Result::Ok(())
}
// error: this impl's bound `I::Item: Tr2` isn't on a type parameter of `W<I>`
