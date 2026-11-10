//! Iterating.

use crate::option::Option;

/// What gives values one at a time, as a `for` loop takes them.
export trait Iterator {
    /// What it gives.
    type Item;

    /// What getting the next value may do.
    effect e;

    /// The next value, or `None` when there are no more.
    fn next(&mut self) -> Option<Self::Item> with Self::e;

    /// Gives `f` of each value. Nothing is called until it's iterated.
    fn map<U, effect fx, F: FnMut(Self::Item) -> U with fx>(self, f: F) -> Map<Self, F> {
        Map { iter: self, f }
    }

    /// The values `p` says `true` of, throwing the rest away. `p` takes each
    /// by `&`, for as long as the call, so `|x: &i64| *x > 0` will do.
    fn filter<effect px, P: FnMut(&Self::Item) -> bool with px>(self, p: P) -> Filter<Self, P>
    where
        Self::Item: Discard,
    { Filter { iter: self, p } }

    /// `f(..f(f(init, a), b).., z)` of the values `a`, `b`, .. `z`. It
    /// throws itself, and `f`, away at the end.
    #[blame_caller]
    fn fold<B, effect fx, F: FnMut(B, Self::Item) -> B with fx + Discard>(
        mut self,
        init: B,
        mut f: F,
    ) -> B with {Self::e, fx} where Self: Discard {
        let mut acc = init;
        loop {
            match self.next() {
                Some(x) => acc = f(acc, x),
                None => return acc,
            }
        }
    }

    /// Each value with its index, counting from 0: `(0, a)`, `(1, b)`, ..
    /// Counting past the greatest `usize` panics.
    fn enumerate(self) -> Enumerate<Self> { Enumerate { iter: self, next: 0 } }

    /// A value of each, in pairs, `(a, x)`, `(b, y)`, .., until either runs
    /// out. A value of `self`'s with none of `other`'s to go with it is
    /// thrown away, so those must be `Discard`.
    fn zip<J: Iterator>(self, other: J) -> Zip<Self, J> where Self::Item: Discard {
        Zip { a: self, b: other }
    }

    /// The values after the first `n`, which are thrown away, so they must
    /// be `Discard`.
    fn skip(self, n: usize) -> Skip<Self> where Self::Item: Discard { Skip { iter: self, n } }

    /// How many values there are, throwing each away. Counting past the
    /// greatest `usize` panics.
    #[blame_caller]
    fn count(self) -> usize with {Self::e, panic} where Self: Discard, Self::Item: Discard {
        self.fold(0, |n, _x| n + 1)
    }

    /// Whether `p` says `true` of any value. It stops at the first it does,
    /// leaving the rest to iterate, and is `false` of none.
    #[blame_caller]
    fn any<effect px, P: FnMut(Self::Item) -> bool with px + Discard>(&mut self, mut p: P) -> bool
    with {Self::e, px}
    {
        loop {
            match self.next() {
                Some(x) => {
                    if p(x) {
                        return true;
                    }
                },
                None => return false,
            }
        }
    }

    /// Whether `p` says `true` of every value. It stops at the first it
    /// doesn't, leaving the rest to iterate, and is `true` of none.
    #[blame_caller]
    fn all<effect px, P: FnMut(Self::Item) -> bool with px + Discard>(&mut self, mut p: P) -> bool
    with {Self::e, px}
    {
        loop {
            match self.next() {
                Some(x) => {
                    if !p(x) {
                        return false;
                    }
                },
                None => return true,
            }
        }
    }

    /// Its values back to front, of an iterator that can give them from
    /// the back (`DoubleEndedIterator`).
    fn rev(self) -> Rev<Self> where Self: DoubleEndedIterator { Rev { iter: self } }
}

/// An iterator that can also give its values from the back: what's left
/// is what's between the two ends, and each value is given once, from
/// whichever end gets to it first.
export trait DoubleEndedIterator: Iterator {
    /// The last value left, or `None` when there are no more.
    fn next_back(&mut self) -> Option<Self::Item> with Self::e;
}

/// Iterating through a mutable reference iterates what it points to, which
/// is still there afterwards: `for x in &mut it`.
impl<'a, I: Iterator> Iterator for &'a mut I {
    type Item = I::Item;
    effect e = I::e;

    #[blame_caller]
    fn next(&mut self) -> Option<I::Item> with I::e { (**self).next() }
}

/// What `Iterator::map` gives: `f` of what `iter` does.
#[derive(
    Discard where I: Discard, F: Discard,
    Send where I: Send, F: Send,
    Sync where I: Sync, F: Sync,
)]
export struct Map<I, F> { iter: I, f: F }

/// Its values are `U`s, what `f` gives, which `F`'s bound decides.
impl<U, effect fx, I: Iterator, F: FnMut(I::Item) -> U with fx> Iterator for Map<I, F> {
    type Item = U;
    effect e = {I::e, fx};

    #[blame_caller]
    fn next(&mut self) -> Option<U> with {I::e, fx} {
        match self.iter.next() {
            Some(x) => Some((self.f)(x)),
            None => None,
        }
    }
}

/// From the back too, `f` of each of what `iter` gives from there.
impl<
    U,
    effect fx,
    I: DoubleEndedIterator,
    F: FnMut(I::Item) -> U with fx,
> DoubleEndedIterator for Map<I, F> {
    #[blame_caller]
    fn next_back(&mut self) -> Option<U> with {I::e, fx} {
        match self.iter.next_back() {
            Some(x) => Some((self.f)(x)),
            None => None,
        }
    }
}

/// What `Iterator::filter` gives: the values of `iter` that `p` says `true`
/// of.
#[derive(
    Discard where I: Discard, P: Discard,
    Send where I: Send, P: Send,
    Sync where I: Sync, P: Sync,
)]
export struct Filter<I, P> { iter: I, p: P }

/// `next` throws away what `p` doesn't keep, so those must be `Discard`.
impl<effect px, I: Iterator, P: FnMut(&I::Item) -> bool with px> Iterator for Filter<I, P>
where
    I::Item: Discard,
{
    type Item = I::Item;
    effect e = {I::e, px};

    #[blame_caller]
    fn next(&mut self) -> Option<I::Item> with {I::e, px} {
        loop {
            match self.iter.next() {
                Some(x) => {
                    if (self.p)(&x) {
                        return Some(x);
                    }
                },
                None => return None,
            }
        }
    }
}

/// What `Iterator::enumerate` gives: each value of `iter`, with its index,
/// `next` the one after the last.
#[derive(Discard where I: Discard, Send where I: Send, Sync where I: Sync)]
export struct Enumerate<I> { iter: I, next: usize }

impl<I: Iterator> Iterator for Enumerate<I> {
    type Item = (usize, I::Item);
    effect e = {I::e, panic};

    #[blame_caller]
    fn next(&mut self) -> Option<(usize, I::Item)> with {I::e, panic} {
        match self.iter.next() {
            Some(x) => {
                let i = self.next;
                self.next = i + 1;
                Some((i, x))
            },
            None => None,
        }
    }
}

/// What `Iterator::zip` gives: a value of `a` and one of `b`, in pairs.
#[derive(
    Discard where A: Discard, B: Discard,
    Send where A: Send, B: Send,
    Sync where A: Sync, B: Sync,
)]
export struct Zip<A, B> { a: A, b: B }

/// `next` throws away a value of `a`'s when `b` has run out, so those must
/// be `Discard`.
impl<A: Iterator, B: Iterator> Iterator for Zip<A, B> where A::Item: Discard {
    type Item = (A::Item, B::Item);
    effect e = {A::e, B::e};

    #[blame_caller]
    fn next(&mut self) -> Option<(A::Item, B::Item)> with {A::e, B::e} {
        match self.a.next() {
            Some(x) => match self.b.next() {
                Some(y) => Some((x, y)),
                None => None,
            },
            None => None,
        }
    }
}

/// What `Iterator::skip` gives: the values of `iter` after the first `n`
/// left to skip.
#[derive(Discard where I: Discard, Send where I: Send, Sync where I: Sync)]
export struct Skip<I> { iter: I, n: usize }

impl<I: Iterator> Iterator for Skip<I> where I::Item: Discard {
    type Item = I::Item;
    effect e = I::e;

    #[blame_caller]
    fn next(&mut self) -> Option<I::Item> with I::e {
        while self.n > 0 {
            self.n = self.n - 1;
            if self.iter.next().is_none() {
                return None;
            }
        }
        self.iter.next()
    }
}

/// What `Iterator::rev` gives: the values of `iter`, back to front.
#[derive(Discard where I: Discard, Send where I: Send, Sync where I: Sync)]
export struct Rev<I> { iter: I }

impl<I: DoubleEndedIterator> Iterator for Rev<I> {
    type Item = I::Item;
    effect e = I::e;

    #[blame_caller]
    fn next(&mut self) -> Option<I::Item> with I::e { self.iter.next_back() }
}

impl<I: DoubleEndedIterator> DoubleEndedIterator for Rev<I> {
    #[blame_caller]
    fn next_back(&mut self) -> Option<I::Item> with I::e { self.iter.next() }
}
