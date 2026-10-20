// Iterating.

use crate::option::Option;

/// What gives values one at a time, as a `for` loop takes them.
pub trait Iterator {
    /// What it gives.
    type Item;

    /// The next value, or `None` when there are no more.
    fn next(&mut self) -> Option<Self::Item>;

    /// Gives `f` of each value.
    fn map<U, F: FnMut(Self::Item) -> U>(self, f: F) -> Map<Self, F, U> { Map { iter: self, f } }

    /// `f(..f(f(init, a), b).., z)` of the values `a`, `b`, .. `z`. It
    /// throws itself, and `f`, away at the end.
    fn fold<B, F: FnMut(B, Self::Item) -> B + Discard>(mut self, init: B, mut f: F) -> B
    where
        Self: Discard,
    {
        let mut acc = init;
        loop {
            match self.next() {
                Option::Some(x) => acc = f(acc, x),
                Option::None => return acc,
            }
        }
    }

    /// How many values there are, throwing each away.
    fn count(self) -> int where Self: Discard, Self::Item: Discard { self.fold(0, |n, _x| n + 1) }
}

/// Iterating through a mutable reference iterates what it points to, which
/// is still there afterwards: `for x in &mut it`.
impl<'a, I: Iterator> Iterator for &'a mut I {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> { (**self).next() }
}

/// What `Iterator::map` gives: `f` of what `iter` does, which are `U`s.
///
/// `U` is a parameter, though no field holds one, because an impl's type
/// must say what each of its parameters is (docs/06-traits.md).
#[derive(Discard where I: Discard, F: Discard)]
pub struct Map<I, F, U> { iter: I, f: F }

impl<U, I: Iterator, F: FnMut(I::Item) -> U> Iterator for Map<I, F, U> {
    type Item = U;

    fn next(&mut self) -> Option<U> {
        match self.iter.next() {
            Option::Some(x) => Option::Some((self.f)(x)),
            Option::None => Option::None,
        }
    }
}
