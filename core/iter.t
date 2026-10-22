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
    fn map<U, f, F: FnMut(Self::Item) -> U with f>(self, f: F) -> Map<Self, F> {
        Map { iter: self, f }
    }

    /// The values `p` says `true` of, throwing the rest away. `p` takes each
    /// by `&`, for as long as the call, so `|x: &int| *x > 0` will do.
    fn filter<p, P: FnMut(&Self::Item) -> bool with p>(self, p: P) -> Filter<Self, P>
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
    /// greatest `usize` panics.
    fn count(self) -> usize with {Self::e, panic} where Self: Discard, Self::Item: Discard {
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

/// What `Iterator::map` gives: `f` of what `iter` does.
#[derive(Discard where I: Discard, F: Discard)]
pub struct Map<I, F> { iter: I, f: F }

/// Its values are `U`s, what `f` gives, which `F`'s bound decides.
impl<U, f, I: Iterator, F: FnMut(I::Item) -> U with f> Iterator for Map<I, F> {
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
/// of.
#[derive(Discard where I: Discard, P: Discard)]
pub struct Filter<I, P> { iter: I, p: P }

/// `next` throws away what `p` doesn't keep, so those must be `Discard`.
impl<p, I: Iterator, P: FnMut(&I::Item) -> bool with p> Iterator for Filter<I, P>
where
    I::Item: Discard,
{
    type Item = I::Item;
    effect e = {I::e, p};

    fn next(&mut self) -> Option<I::Item> with {I::e, p} {
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
