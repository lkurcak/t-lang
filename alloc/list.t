//! Lists: dense, resizable arrays.

use core::slice::{Iter, IterMut};

use crate::raw::RawBuf;

/// A list of `T`s, one after another in a buffer on the heap, which grows
/// as values are pushed. `get` sees a value by its index, from `0` up to,
/// not including, `len()`, and `as_slice` sees them all, as a slice.
///
/// It's `Discard` if its values are: dropping it drops each of them, front
/// to back, and frees its buffer. One of values that aren't is used up by
/// `free_with`.
///
/// # Invariant
///
/// Slots `0..len` of `buf` hold values, and the rest don't. Every `unsafe`
/// block here counts on it, and every method keeps it. Its fields are
/// private, so nothing else can break it. That `len <= buf.cap()` is a
/// refinement, which the checker proves each method keeps.
export struct List<T> { buf: RawBuf<T>, len: { n: usize | n <= buf.cap() } }

impl<T> List<T> {
    /// An empty list. It allocates nothing until something's pushed.
    export fn new() -> { l: List<T> | l.len() == 0 } { List { buf: RawBuf::empty(), len: 0 } }

    /// An empty list with room for `cap` values before it grows.
    #[blame_caller]
    export fn with_capacity(cap: usize) -> { l: List<T> | l.len() == 0 } with alloc {
        List { buf: RawBuf::new(cap), len: 0 }
    }

    /// How many values it holds. It's a measure, so refinements can say
    /// it: `l[i]` needs `i < l.len()`.
    #[measure]
    export fn len(&self) -> usize { self.len }

    /// Does it hold none?
    #[measure]
    export fn is_empty(&self) -> bool { self.len() == 0 }

    /// How many values it has room for before it grows.
    #[measure]
    export fn capacity(&self) -> usize { self.buf.cap() }

    /// Puts `v` at the end. When there's no room, it doubles the buffer
    /// first, which moves its values somewhere new. A list as long as a
    /// `usize` can count has no room, and its buffer can't double, which
    /// aborts, as running out of memory does.
    #[blame_caller]
    export fn push(&mut self, v: T) with alloc ensures self.len() == old(self.len()) + 1 {
        let cap = self.buf.cap();
        if self.len == cap {
            self.buf.grow(if cap == 0 { 4 } else { cap });
        }
        // SAFETY: slot `len` is past those that hold values, so it holds
        // none.
        unsafe { self.buf.write(self.len, v) };
        self.len = self.len + 1;
    }

    /// Takes the last value out, or `None` if it's empty.
    export fn pop(&mut self) -> Option<T>
    ensures
        self.len() + 1 == old(self.len()) || (self.len() == 0 && old(self.len()) == 0),
    {
        if self.len == 0 {
            return Option::None;
        }
        self.len = self.len - 1;
        // SAFETY: slot `len` held the last value, and is past those that
        // hold values now.
        Option::Some(unsafe { self.buf.read(self.len) })
    }

    /// Its values, as a slice, as many as it holds.
    export fn as_slice(&self) -> { s: &[T] | s.len() == self.len() } {
        // SAFETY: slots `0..len` hold values.
        unsafe { self.buf.slice(0, self.len) }
    }

    /// Its values, as a slice to change, as many as it holds.
    export fn as_mut_slice(&mut self) -> { s: &mut [T] | s.len() == old(self.len()) } {
        // SAFETY: slots `0..len` hold values.
        unsafe { self.buf.slice_mut(0, self.len) }
    }

    /// The value at `i`, which is less than `len()`, so there's one: `l[i]`
    /// is `*l.index(i)`.
    export fn index(&self, i: { i: usize | i < self.len() }) -> &T { &self.as_slice()[i] }

    /// The value at `i`, which is less than `len()`, to change: `l[i] = v`
    /// is `*l.index_mut(i) = v`.
    export fn index_mut(&mut self, i: { i: usize | i < self.len() }) -> &mut T {
        &mut self.as_mut_slice()[i]
    }

    /// The value at `i`, or `None` if there's none: if `i` isn't less than
    /// `len()`.
    export fn get(&self, i: usize) -> Option<&T> { self.as_slice().get(i) }

    /// The value at `i`, to change, or `None` if there's none.
    export fn get_mut(&mut self, i: usize) -> Option<&mut T> { self.as_mut_slice().get_mut(i) }

    /// Its values, by reference, front to back.
    export fn iter(&self) -> Iter<T> { self.as_slice().iter() }

    /// Its values, to change, front to back.
    export fn iter_mut(&mut self) -> IterMut<T> { self.as_mut_slice().iter_mut() }

    /// Swaps the values at `i` and `j`, if there are values there, and
    /// says whether it did.
    export fn swap(&mut self, i: usize, j: usize) -> bool ensures self.len() == old(self.len()) {
        // SAFETY: slots `0..len` hold values. The slice borrows `buf`, not
        // the list, so the checker sees `len` stay as it was.
        let s = unsafe { self.buf.slice_mut(0, self.len) };
        s.swap(i, j)
    }

    /// Gives each value, front to back, to `f`, and then the buffer back.
    export fn free_with<effect e, F: FnMut(T) with e + Discard>(mut self, mut f: F) with e {
        let mut i: usize = 0;
        while i < self.len {
            // SAFETY: slots `i..len` still hold values, and slot `i` is
            // read only this once.
            f(unsafe { self.buf.read(i) });
            i = i + 1;
        }
        // SAFETY: the values in slots `0..len` have been read out, and the
        // slots after them never held any, so every slot is empty.
        unsafe { self.buf.free() };
    }
}

/// Drops its values, front to back, and gives the buffer back.
impl<T: Discard> Discard for List<T> {
    fn discard(self) { self.free_with(|_x| {}) }
}

impl<T: Discard> List<T> {
    /// Throws away the values from index `n` on, if there are any, so it
    /// holds at most `n`. It's `pop` until then, so safe code.
    export fn truncate(&mut self, n: usize) ensures self.len() <= n {
        while self.len > n {
            self.pop();
        }
    }

    /// Throws all its values away, keeping the buffer for more.
    export fn clear(&mut self) ensures self.len() == 0 { self.truncate(0) }
}
