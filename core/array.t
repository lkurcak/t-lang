//! Arrays: a number of values fixed by their type, inline.

use crate::ops::Range;
use crate::option::Option;
use crate::slice::{Iter, IterMut};

/// `[T; N]`: exactly `N` values of type `T`, one after another, inline,
/// where the array is: in a local, a field, or another array. How many is
/// in its type, so nothing can change it, and `len()` is `N` wherever it's
/// asked.
///
/// What's trusted is its intrinsics: `len`, which is `N`, and `as_slice`
/// and `as_mut_slice`, each a slice of all its values. The rest is safe
/// code over them, and over slices'.
impl<T, const N: usize> [T; N] {
    /// How many values it has: `N`. It's a measure, so refinements can say
    /// it: `i: { i: usize | i < a.len() }`.
    #[intrinsic]
    #[measure]
    export fn len(&self) -> usize;

    /// Has it none?
    #[measure]
    export fn is_empty(&self) -> bool { self.len() == 0 }

    /// All its values, as a slice.
    #[intrinsic]
    export fn as_slice(&self) -> { s: &[T] | s.len() == self.len() };

    /// All its values, as a slice to change them through.
    #[intrinsic]
    export fn as_mut_slice(&mut self) -> { s: &mut [T] | s.len() == old(self.len()) };

    /// The value at `i`, which is less than `len()`, so there's one: `a[i]`
    /// is `*a.index(i)`.
    export fn index(&self, i: { i: usize | i < self.len() }) -> &T {
        let s = self.as_slice();
        &s[i]
    }

    /// The value at `i`, which is less than `len()`, to change: `a[i] = v`
    /// is `*a.index_mut(i) = v`.
    export fn index_mut(&mut self, i: { i: usize | i < self.len() }) -> &mut T {
        let s = self.as_mut_slice();
        &mut s[i]
    }

    /// Its values from index `r.start()` up to, not including, `r.end()`,
    /// as a slice: `a.slice(1..3)`. The range is in it, so they're there.
    export fn slice<'a>(
        &'a self,
        r: { r: Range<usize> | r.start() <= r.end() && r.end() <= self.len() },
    ) -> { t: &'a [T] | t.len() + r.start() == r.end() } { self.as_slice().slice(r) }

    /// Its values from index `r.start()` up to, not including, `r.end()`,
    /// as a slice to change them through.
    export fn slice_mut<'a>(
        &'a mut self,
        r: { r: Range<usize> | r.start() <= r.end() && r.end() <= self.len() },
    ) -> { t: &'a mut [T] | t.len() + r.start() == r.end() } { self.as_mut_slice().slice_mut(r) }

    /// The value at `i`, or `None` if there's none: if `i` isn't less than
    /// `len()`.
    export fn get(&self, i: usize) -> Option<&T> { self.as_slice().get(i) }

    /// The value at `i`, to change, or `None` if there's none.
    export fn get_mut(&mut self, i: usize) -> Option<&mut T> { self.as_mut_slice().get_mut(i) }

    /// Its values, by reference, front to back.
    export fn iter(&self) -> Iter<T> { self.as_slice().iter() }

    /// Its values, to change, front to back.
    export fn iter_mut(&mut self) -> IterMut<T> { self.as_mut_slice().iter_mut() }
}

/// An array whose value at each index `i` is `f(i)`, made front to back:
/// so its values needn't be `Copy`. It does what `f` does.
export fn from_fn<T, const N: usize, effect e, F: FnMut(usize) -> T with e + Discard>(
    f: F,
) -> [T; N] with e {
    let mut f = f;
    from_fn_mut(&mut f)
}

/// `from_fn`, calling `f` where it is: so dropping it is `from_fn`'s, not
/// the intrinsic's.
#[intrinsic]
fn from_fn_mut<T, const N: usize, effect e, F: FnMut(usize) -> T with e>(f: &mut F) -> [T; N]
with e;
