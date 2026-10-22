// Lists: dense, resizable arrays.

use core::slice::{Iter, IterMut};

use crate::raw::RawBuf;

/// A list of `T`s, one after another in a buffer on the heap, which grows
/// as values are pushed. `get` sees a value by its index, from `0` up to,
/// not including, `len()`, and `as_slice` sees them all, as a slice.
///
/// It isn't `Discard`, as its buffer isn't: it's used up by `free`, or by
/// `free_with` when its values aren't `Discard` either.
///
/// # Invariant
///
/// Slots `0..len` of `buf` hold values, and the rest don't, so `len <=
/// buf.cap()`. Every `unsafe` block here counts on it, and every method
/// keeps it. Its fields are private, so nothing else can break it.
pub struct List<T> { buf: RawBuf<T>, len: { l: int | l >= 0 } }

impl<T> List<T> {
    /// An empty list. It allocates nothing until something's pushed.
    pub fn new() -> List<T> { List { buf: RawBuf::empty(), len: 0 } }

    /// An empty list with room for `cap` values before it grows.
    pub fn with_capacity(cap: { c: int | c >= 0 }) -> List<T> with {alloc, panic} {
        List { buf: RawBuf::new(cap), len: 0 }
    }

    /// How many values it holds.
    pub fn len(&self) -> { n: int | n >= 0 } { self.len }

    /// Does it hold none?
    pub fn is_empty(&self) -> bool { self.len == 0 }

    /// How many values it has room for before it grows.
    pub fn capacity(&self) -> { c: int | c >= 0 } { self.buf.cap() }

    /// Puts `v` at the end. When there's no room, it doubles the buffer
    /// first, which moves its values somewhere new.
    pub fn push(&mut self, v: T) with {alloc, panic} {
        let cap = self.buf.cap();
        if self.len == cap {
            if cap == 0 {
                self.buf.grow(4);
            } else {
                self.buf.grow(cap * 2);
            }
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

    /// Its values, as a slice.
    pub fn as_slice(&self) -> &[T] {
        // SAFETY: slots `0..len` hold values, and `len <= cap()`.
        unsafe { self.buf.slice(0, self.len) }
    }

    /// Its values, as a slice to change.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        // SAFETY: slots `0..len` hold values, and `len <= cap()`. Nothing
        // done through the slice can take one out.
        unsafe { self.buf.slice_mut(0, self.len) }
    }

    /// The value at `i`, or `None` if there's none: if `i` is negative, or
    /// not less than `len()`.
    pub fn get(&self, i: int) -> Option<&T> { self.as_slice().get(i) }

    /// The value at `i`, to change, or `None` if there's none.
    pub fn get_mut(&mut self, i: int) -> Option<&mut T> { self.as_mut_slice().get_mut(i) }

    /// Its values, by reference, front to back.
    pub fn iter(&self) -> Iter<T> { self.as_slice().iter() }

    /// Its values, to change, front to back.
    pub fn iter_mut(&mut self) -> IterMut<T> { self.as_mut_slice().iter_mut() }

    /// Swaps the values at `i` and `j`, if there are values there, and
    /// says whether it did.
    pub fn swap(&mut self, i: int, j: int) -> bool { self.as_mut_slice().swap(i, j) }

    /// Gives each value, front to back, to `f`, and then the buffer back.
    pub fn free_with<e, F: FnMut(T) with e + Discard>(mut self, mut f: F) with e {
        let mut i: { i: int | i >= 0 } = 0;
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

impl<T: Discard> List<T> {
    /// Throws its values away, and gives the buffer back.
    pub fn free(self) { self.free_with(|_x| {}) }

    /// Throws away the values from index `n` on, if there are any, so it
    /// holds at most `n`. It's `pop` until then, so safe code.
    pub fn truncate(&mut self, n: { n: int | n >= 0 }) {
        while self.len > n {
            self.pop();
        }
    }

    /// Throws all its values away, keeping the buffer for more.
    pub fn clear(&mut self) { self.truncate(0) }
}
