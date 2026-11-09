//! Slices: values one after another in memory, seen through a reference.

use crate::cmp::{Ord, Ordering};
use crate::iter::Iterator;
use crate::mem;
use crate::ops::Range;
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
        if n == 0 { None } else { self.get(n - 1) }
    }

    /// The values before index `mid`, which is at most `len()`, and those
    /// from it on: `split_at` and `slice` are safe code over it.
    #[intrinsic]
    fn cut<'a>(
        &'a self,
        mid: { m: usize | m <= self.len() },
    ) -> { s: (&'a [T], &'a [T]) | s.0.len() == mid && s.1.len() + mid == self.len() };

    /// The values before index `mid`, which is at most `len()`, and those
    /// from it on, each to change. They share no values, so both can be
    /// changed at once.
    #[intrinsic]
    fn cut_mut<'a>(
        &'a mut self,
        mid: { m: usize | m <= self.len() },
    ) -> { s: (&'a mut [T], &'a mut [T]) | s.0.len() == mid && s.1.len() + mid == self.len() };

    /// The values before index `mid` and those from it on, or `None` if
    /// `mid` is more than `len()`.
    export fn split_at<'a>(
        &'a self,
        mid: usize,
    ) -> Option<{ s: (&'a [T], &'a [T]) | s.0.len() == mid && s.1.len() + mid == self.len() }> {
        if mid <= self.len() { Some(self.cut(mid)) } else { None }
    }

    /// The values before index `mid` and those from it on, each to change,
    /// or `None` if `mid` is more than `len()`. They share no values, so
    /// both can be changed at once.
    export fn split_at_mut<'a>(
        &'a mut self,
        mid: usize,
    ) -> Option<
        { s: (&'a mut [T], &'a mut [T]) | s.0.len() == mid && s.1.len() + mid == self.len() },
    > {
        if mid <= self.len() { Some(self.cut_mut(mid)) } else { None }
    }

    /// The values from index `r.start()` up to, not including, `r.end()`:
    /// `s.slice(1..3)` is the second and the third. The range is in it, so
    /// they're there.
    export fn slice<'a>(
        &'a self,
        r: { r: Range<usize> | r.start() <= r.end() && r.end() <= self.len() },
    ) -> { t: &'a [T] | t.len() + r.start() == r.end() } {
        let head: &'a [T] = self.cut(r.end()).0;
        head.cut(r.start()).1
    }

    /// The values from index `r.start()` up to, not including, `r.end()`,
    /// to change.
    export fn slice_mut<'a>(
        &'a mut self,
        r: { r: Range<usize> | r.start() <= r.end() && r.end() <= self.len() },
    ) -> { t: &'a mut [T] | t.len() + r.start() == r.end() } {
        let head: &'a mut [T] = self.cut_mut(r.end()).0;
        head.cut_mut(r.start()).1
    }

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
            Some((head, tail)) => match head.get_mut(lo) {
                Some(a) => match tail.get_mut(0) {
                    Some(b) => {
                        mem::swap(a, b);
                        true
                    },
                    None => false,
                },
                None => false,
            },
            None => false,
        }
    }

    /// Its values, by reference, front to back.
    export fn iter(&self) -> Iter<T> { Iter { rest: self } }

    /// Its values, to change, front to back.
    export fn iter_mut(&mut self) -> IterMut<T> { IterMut { rest: self } }
}

impl<T: Eq> [T] {
    /// Is one of its values equal to `x`?
    export fn contains(&self, x: &T) -> bool {
        for y in self.iter() {
            if *y == *x {
                return true;
            }
        }
        false
    }
}

/// Slices value by value, the first that differ deciding, and a slice
/// before a longer one it starts: the order of words in a dictionary.
impl<T: Ord> Ord for [T] {
    fn cmp(&self, other: &[T]) -> Ordering {
        let n = self.len();
        let m = other.len();
        let mut i = 0usize;
        while i < n && i < m {
            match self[i].cmp(&other[i]) {
                Ordering::Equal => {},
                o => return o,
            }
            i = i + 1;
        }
        n.cmp(&m)
    }
}

/// A slice with no values, of any lifetime: it points at nothing.
#[intrinsic]
export fn empty<'a, T>() -> &'a [T];

/// A slice with no values, to change, of any lifetime: it points at
/// nothing, so nothing else can see what it does.
#[intrinsic]
export fn empty_mut<'a, T>() -> &'a mut [T];

/// What `iter` gives: a reference to each value of `rest`, front to back.
#[derive(Discard, Send where T: Sync, Sync where T: Sync)]
export struct Iter<'a, T> { rest: &'a [T] }

/// Safe code over `split_at`: each value is the head of what's left.
impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;
    effect e = {};

    fn next(&mut self) -> Option<&'a T> {
        let rest: &'a [T] = self.rest;
        match rest.split_at(1) {
            Some((head, tail)) => {
                self.rest = tail;
                head.get(0)
            },
            None => None,
        }
    }
}

/// What `iter_mut` gives: a reference to change each value of `rest`,
/// front to back.
#[derive(Discard, Send where T: Send, Sync where T: Sync)]
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
            Some((head, tail)) => {
                self.rest = tail;
                head.get_mut(0)
            },
            None => None,
        }
    }
}
