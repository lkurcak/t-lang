// A law's trait implemented for a type whose impl of the supertrait is
// another package's, whose body isn't checked here: refused, not trusted
// (docs/88-iterator-laws.md).
trait Short: Iterator {
    law fn next(&mut self) -> { r: Option<Self::Item> | r.is_none() } with Self::e;
}

impl<'a, T> Short for core::slice::Iter<'a, T> {}

fn main() {}
// error: can't check that `Iter<'a, T>` keeps `Short`'s law of `next`: its impl of `Iterator` is another package's, whose bodies are checked there
