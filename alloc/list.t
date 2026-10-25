//! Lists: dense, resizable arrays.

use core::num::greatest;
use core::slice::{Iter, IterMut};

use crate::raw::{RawBuf, out_of_memory};

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
/// Slots `0..len` of `buf` hold values, and the rest don't, so `len <=
/// buf.cap()`. Every `unsafe` block here counts on it, and every method
/// keeps it. Its fields are private, so nothing else can break it.
pub struct List<T> { buf: RawBuf<T>, len: usize }

impl<T> List<T> {
    /// An empty list. It allocates nothing until something's pushed.
    pub fn new() -> List<T> { List { buf: RawBuf::empty(), len: 0 } }

    /// An empty list with room for `cap` values before it grows.
    pub fn with_capacity(cap: usize) -> List<T> with alloc {
        List { buf: RawBuf::new(cap), len: 0 }
    }

    /// How many values it holds. It's a measure, so refinements can say
    /// it: `l[i]` needs `i < l.len()`.
    #[measure]
    pub fn len(&self) -> usize { self.len }

    /// Does it hold none?
    #[measure]
    pub fn is_empty(&self) -> bool { self.len == 0 }

    /// How many values it has room for before it grows.
    #[measure]
    pub fn capacity(&self) -> usize { self.buf.cap() }

    /// Puts `v` at the end. When there's no room, it doubles the buffer
    /// first, which moves its values somewhere new.
    pub fn push(&mut self, v: T) with alloc {
        let cap = self.buf.cap();
        if self.len == cap {
            self.buf.grow(if cap == 0 { 4 } else { cap });
        }
        // A list as long as a `usize` can count can't count one more value.
        if self.len == greatest() {
            out_of_memory();
        }
        // SAFETY: slot `len` is past those that hold values, so it holds
        // none, and it's less than `cap()`, which is more than `len` now.
        unsafe { self.buf.write(self.len, v) };
        self.len = self.len + 1;
    }

    /// Takes the last value out, or `None` if it's empty.
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return Option::None;
        }
        self.len = self.len - 1;
        // SAFETY: slot `len` held the last value, and is past those that
        // hold values now.
        Option::Some(unsafe { self.buf.read(self.len) })
    }

    /// Its values, as a slice, as many as it holds.
    pub fn as_slice(&self) -> { s: &[T] | s.len() == self.len() } {
        // SAFETY: slots `0..len` hold values, and `len <= cap()`.
        unsafe { self.buf.slice(0, self.len) }
    }

    /// Its values, as a slice to change, as many as it holds.
    pub fn as_mut_slice(&mut self) -> { s: &mut [T] | s.len() == self.len() } {
        // SAFETY: slots `0..len` hold values, and `len <= cap()`. Nothing
        // done through the slice can take one out.
        unsafe { self.buf.slice_mut(0, self.len) }
    }

    /// The value at `i`, which is less than `len()`, so there's one: `l[i]`
    /// is `*l.index(i)`.
    pub fn index(&self, i: { i: usize | i < self.len() }) -> &T { &self.as_slice()[i] }

    /// The value at `i`, which is less than `len()`, to change: `l[i] = v`
    /// is `*l.index_mut(i) = v`.
    pub fn index_mut(&mut self, i: { i: usize | i < self.len() }) -> &mut T {
        // SAFETY: slots `0..len` hold values, and `len <= cap()`. Nothing
        // done through the slice can take one out.
        let s = unsafe { self.buf.slice_mut(0, self.len) };
        &mut s[i]
    }

    /// The value at `i`, or `None` if there's none: if `i` isn't less than
    /// `len()`.
    pub fn get(&self, i: usize) -> Option<&T> { self.as_slice().get(i) }

    /// The value at `i`, to change, or `None` if there's none.
    pub fn get_mut(&mut self, i: usize) -> Option<&mut T> { self.as_mut_slice().get_mut(i) }

    /// Its values, by reference, front to back.
    pub fn iter(&self) -> Iter<T> { self.as_slice().iter() }

    /// Its values, to change, front to back.
    pub fn iter_mut(&mut self) -> IterMut<T> { self.as_mut_slice().iter_mut() }

    /// Swaps the values at `i` and `j`, if there are values there, and
    /// says whether it did.
    pub fn swap(&mut self, i: usize, j: usize) -> bool { self.as_mut_slice().swap(i, j) }

    /// Gives each value, front to back, to `f`, and then the buffer back.
    pub fn free_with<effect e, F: FnMut(T) with e + Discard>(mut self, mut f: F) with e {
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
    pub fn truncate(&mut self, n: usize) {
        while self.len > n {
            self.pop();
        }
    }

    /// Throws all its values away, keeping the buffer for more.
    pub fn clear(&mut self) { self.truncate(0) }
}
