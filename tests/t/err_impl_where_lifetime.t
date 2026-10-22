// As a function's can't, an impl's `where` clause can't say what outlives
// what.
trait Tr {
    fn a(&self) -> i64;
}

#[derive(Discard where T: Discard)]
struct W<T> { t: T }

impl<T> Tr for W<T>
where
    T: 'static,
{
    fn a(&self) -> i64 { 1 }
}

fn main() {}
// error: a `where` clause can't say what outlives what
