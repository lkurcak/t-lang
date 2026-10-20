// An impl's `where` clause can't bound the impl's own type: that's what
// the impl is about.
trait Tr {
    fn a(&self) -> int;
}

#[derive(Discard where T: Discard)]
struct W<T> { t: T }

impl<T> Tr for W<T>
where
    Self: Discard,
{
    fn a(&self) -> int { 1 }
}

fn main() {}
// error: an impl's `where` clause can only bound a type parameter, or an associated type of one, not `Self`
