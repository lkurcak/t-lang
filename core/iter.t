// Iterating.

use crate::option::Option;

/// What gives values one at a time, as a `for` loop takes them.
pub trait Iterator {
    /// What it gives.
    type Item;

    /// What getting the next value may do.
    effect e;

    /// The next value, or `None` when there are no more.
    fn next(&mut self) -> Option<Self::Item> with Self::e;

    /// Gives `f` of each value. Nothing is called until it's iterated.
    fn map<U, f, F: FnMut(Self::Item) -> U with f>(self, f: F) -> Map<Self, F, U> {
        Map { iter: self, f }
    }

    /// The values `p` says `true` of, throwing the rest away. `p` takes each
    /// by `&`, for as long as the call, so `|x: &int| *x > 0` will do.
    fn filter<p, P: FnMut(&Self::Item) -> bool with p>(self, p: P) -> Filter<Self, P, Self::Item>
    where
        Self::Item: Discard,
    { Filter { iter: self, p } }

    /// `f(..f(f(init, a), b).., z)` of the values `a`, `b`, .. `z`. It
    /// throws itself, and `f`, away at the end.
    fn fold<B, f, F: FnMut(B, Self::Item) -> B with f + Discard>(mut self, init: B, mut f: F) -> B
    with {Self::e, f}
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

    /// How many values there are, throwing each away. Counting past the
    /// greatest `int` panics.
    fn count(self) -> int with {Self::e, panic} where Self: Discard, Self::Item: Discard {
        self.fold(0, |n, _x| n + 1)
    }
}

/// Iterating through a mutable reference iterates what it points to, which
/// is still there afterwards: `for x in &mut it`.
impl<'a, I: Iterator> Iterator for &'a mut I {
    type Item = I::Item;
    effect e = I::e;

    fn next(&mut self) -> Option<I::Item> with I::e { (**self).next() }
}

/// What `Iterator::map` gives: `f` of what `iter` does, which are `U`s.
///
/// `U` is a parameter, though no field holds one, because an impl's type
/// must say what each of its parameters is (docs/06-traits.md).
#[derive(Discard where I: Discard, F: Discard)]
pub struct Map<I, F, U> { iter: I, f: F }

impl<U, f, I: Iterator, F: FnMut(I::Item) -> U with f> Iterator for Map<I, F, U> {
    type Item = U;
    effect e = {I::e, f};

    fn next(&mut self) -> Option<U> with {I::e, f} {
        match self.iter.next() {
            Option::Some(x) => Option::Some((self.f)(x)),
            Option::None => Option::None,
        }
    }
}

/// What `Iterator::filter` gives: the values of `iter` that `p` says `true`
/// of, which are `T`s, as `iter`'s are.
///
/// `T` is a parameter for the impl to say it's `Discard`, as `next` throws
/// away what `p` doesn't keep.
#[derive(Discard where I: Discard, P: Discard)]
pub struct Filter<I, P, T> { iter: I, p: P }

impl<T: Discard, p, I: Iterator<Item = T>, P: FnMut(&T) -> bool with p> Iterator for Filter<
    I,
    P,
    T,
> {
    type Item = T;
    effect e = {I::e, p};

    fn next(&mut self) -> Option<T> with {I::e, p} {
        loop {
            match self.iter.next() {
                Option::Some(x) => {
                    if (self.p)(&x) {
                        return Option::Some(x);
                    }
                },
                Option::None => return Option::None,
            }
        }
    }
}
