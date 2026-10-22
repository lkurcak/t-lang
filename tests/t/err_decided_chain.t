// Only a bound on a parameter of the impl's type decides another: `U`
// would be decided through `T`, which is decided itself. (For now.)
#[derive(Discard where I: Discard)]
struct Flat<I> { iter: I }

trait Tr {
    fn go(&self) -> i64;
}

impl<T: Iterator<Item = U>, U, I: Iterator<Item = T>> Tr for Flat<I> {
    fn go(&self) -> i64 { 1 }
}

fn main() {}
// error: type parameter `U` isn't in `Flat<I>`, nor decided by a bound
