//! Slices: values one after another in memory, seen through a reference.

use crate::iter::Iterator;
use crate::mem;
use crate::option::Option;

/// `[T]`: some number of `T`s one after another, seen through a reference,
/// `&[T]` or `&mut [T]`, which knows how many. A slice is never a value of
/// its own, so a type parameter never stands for one.
///
/// What's trusted is its intrinsics: each checks the index it's given
/// against the slice's length, so none has an obligation, and
/// `split_at_mut` gives two slices that share no values. The rest is safe
/// code over them.
impl<T> [T] {
    /// How many values it has. It's a measure, so refinements can say it:
    /// `i: { i: usize | i < s.len() }`. It's of the reference, which knows
    /// it, so nothing written through a `&mut [T]` changes it.
    #[intrinsic]
    #[measure]
    export fn len(&self) -> usize;

    /// Has it none?
    #[measure]
    export fn is_empty(&self) -> bool { self.len() == 0 }

    /// The value at `i`, which is less than `len()`, so there's one: `s[i]`
    /// is `*s.index(i)`.
    #[intrinsic]
    export fn index(&self, i: { i: usize | i < self.len() }) -> &T;

    /// The value at `i`, which is less than `len()`, to change: `s[i] = v`
    /// is `*s.index_mut(i) = v`.
    #[intrinsic]
    export fn index_mut(&mut self, i: { i: usize | i < self.len() }) -> &mut T;

    /// The value at `i`, or `None` if there's none: if `i` isn't less than
    /// `len()`.
    #[intrinsic]
    export fn get(&self, i: usize) -> Option<&T>;

    /// The value at `i`, to change, or `None` if there's none.
    #[intrinsic]
    export fn get_mut(&mut self, i: usize) -> Option<&mut T>;

    /// The first value, or `None` if it's empty.
    export fn first(&self) -> Option<&T> { self.get(0) }

    /// The last value, or `None` if it's empty.
    export fn last(&self) -> Option<&T> {
        let n = self.len();
        if n == 0 { Option::None } else { self.get(n - 1) }
    }

    /// The values before index `mid` and those from it on, or `None` if
    /// `mid` is more than `len()`.
    #[intrinsic]
    export fn split_at<'a>(&'a self, mid: usize) -> Option<Split<'a, T>>;

    /// The values before index `mid` and those from it on, each to change,
    /// or `None` if `mid` is more than `len()`. They share no values, so
    /// both can be changed at once.
    #[intrinsic]
    export fn split_at_mut<'a>(&'a mut self, mid: usize) -> Option<SplitMut<'a, T>>;

    /// Swaps the values at `i` and `j`, if there are values there, and says
    /// whether it did. It's `split_at_mut` between them, so safe code.
    export fn swap(&mut self, i: usize, j: usize) -> bool {
        if i >= self.len() || j >= self.len() {
            return false;
        }
        if i == j {
            return true;
        }
        let lo = if i < j { i } else { j };
        let hi = if i < j { j } else { i };
        match self.split_at_mut(hi) {
            Option::Some(SplitMut { head, tail }) => match head.get_mut(lo) {
                Option::Some(a) => match tail.get_mut(0) {
                    Option::Some(b) => {
                        mem::swap(a, b);
                        true
                    },
                    Option::None => false,
                },
                Option::None => false,
            },
            Option::None => false,
        }
    }

    /// Its values, by reference, front to back.
    export fn iter(&self) -> Iter<T> { Iter { rest: self } }

    /// Its values, to change, front to back.
    export fn iter_mut(&mut self) -> IterMut<T> { IterMut { rest: self } }
}

/// A slice with no values, of any lifetime: it points at nothing.
#[intrinsic]
export fn empty<'a, T>() -> &'a [T];

/// A slice with no values, to change, of any lifetime: it points at
/// nothing, so nothing else can see what it does.
#[intrinsic]
export fn empty_mut<'a, T>() -> &'a mut [T];

/// What `split_at` gives: the values before the index it's given, and
/// those from it on.
#[derive(Copy, Discard)]
export struct Split<'a, T> { export head: &'a [T], export tail: &'a [T] }

/// What `split_at_mut` gives: the values before the index it's given, and
/// those from it on, each to change.
#[derive(Discard)]
export struct SplitMut<'a, T> { export head: &'a mut [T], export tail: &'a mut [T] }

/// What `iter` gives: a reference to each value of `rest`, front to back.
#[derive(Discard)]
export struct Iter<'a, T> { rest: &'a [T] }

/// Safe code over `split_at`: each value is the head of what's left.
impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;
    effect e = {};

    fn next(&mut self) -> Option<&'a T> {
        let rest: &'a [T] = self.rest;
        match rest.split_at(1) {
            Option::Some(Split { head, tail }) => {
                self.rest = tail;
                head.get(0)
            },
            Option::None => Option::None,
        }
    }
}

/// What `iter_mut` gives: a reference to change each value of `rest`,
/// front to back.
#[derive(Discard)]
export struct IterMut<'a, T> { rest: &'a mut [T] }

/// Safe code over `split_at_mut`: it takes what's left out of the
/// iterator, leaving an empty slice, and keeps the tail. So each value it
/// gives is in no slice it has any more, and nothing it gives later can
/// be that value too.
impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;
    effect e = {};

    fn next(&mut self) -> Option<&'a mut T> {
        let rest: &'a mut [T] = mem::replace(&mut self.rest, empty_mut());
        match rest.split_at_mut(1) {
            Option::Some(SplitMut { head, tail }) => {
                self.rest = tail;
                head.get_mut(0)
            },
            Option::None => Option::None,
        }
    }
}
