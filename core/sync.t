//! Sharing values between threads: the two traits that say what may be
//! handed to another thread or shared with one, and atomics, which can be
//! changed by several at once (see docs/38-statics.md).
//!
//! There are no threads yet, but statics are already shared by every
//! function that names them, so what's in one must be `Sync`, and changes
//! only atomically.

use crate::num::Int;
use crate::result::Result;

/// Says a type's values may be handed to another thread: moved there, to
/// be used by it alone. What isn't is what shares something that isn't
/// `Sync`: a `&Cell<i32>` handed over would let two threads change the
/// cell at once.
///
/// The integers, `bool`, `char`, `str` and functions are `Send`; a `&T`
/// is if `T` is `Sync`, a `&mut T` if `T` is `Send`, and boxes, arrays
/// and slices if what they hold is. A closure is if what it captures is.
/// A struct or enum is if it says so, with `#[derive(Send)]`, and its
/// fields are: it's only ever derived, like `Copy`, so that whether a type
/// is `Send` follows from its fields alone.
#[intrinsic]
export trait Send {}

/// Says a type's values may be shared with another thread, by `&`:
/// nothing in one changes through a `&`, or only atomically. A `Cell`
/// isn't, nor is an arena; an `Atomic` is.
///
/// The integers, `bool`, `char`, `str` and functions are `Sync`; a `&T`
/// or `&mut T` is if `T` is, and boxes, arrays and slices if what they
/// hold is. A closure is if what it captures is. A struct or enum is if it
/// says so, with `#[derive(Sync)]`, and its fields are.
#[intrinsic]
export trait Sync {}

/// Says values of a type can be atomic: the integers and `bool`, what the
/// machine reads and writes in one step. It's sealed: they're all there
/// are.
export sealed trait Atom: Copy + Eq {}

impl<T: Int> Atom for T {}

impl Atom for bool {}

/// A value that can be changed through a shared reference, `&Atomic<T>`,
/// by several threads at once: each change is one step, which no thread
/// sees half of, and all threads see all of them in one order, the same
/// for all (they're *sequentially consistent*; there are no weaker
/// orderings).
///
/// Like a `Cell`, nothing ever points into it: what's in it is only copied
/// out, and the checker's logic never looks inside, as another thread may
/// change it between any two steps. It's invariant in `T`, and never
/// `Copy`: a copy would be a second atomic, which a change to the first
/// doesn't change.
#[derive(Discard where T: Discard, Send where T: Send, Sync where T: Sync)]
export struct Atomic<T: Atom> { value: T }

impl<T: Atom> Atomic<T> {
    /// An atomic holding `value`.
    export fn new(value: T) -> Atomic<T> { Atomic { value } }

    /// What's in it.
    #[intrinsic]
    export fn load(&self) -> T;

    /// Puts `value` in it.
    export fn store(&self, value: T) {
        let _ = self.swap(value);
    }

    /// Puts `value` in it, and gives back what was there.
    #[intrinsic]
    export fn swap(&self, value: T) -> T;

    /// Puts `new` in it if what's there is `current`, and gives back what
    /// was there, either way: whether it changed is whether that's
    /// `current`.
    #[intrinsic]
    export fn compare_swap(&self, current: T, new: T) -> T;

    /// Puts `new` in it if what's there is `current`: `Ok` with what was
    /// there if it did, `Err` with what's there if it didn't.
    export fn compare_exchange(&self, current: T, new: T) -> Result<T, T> {
        let old = self.compare_swap(current, new);
        match old == current {
            true => Result::Ok(old),
            false => Result::Err(old),
        }
    }

    /// Puts what's there `&` `value` in it, and gives back what was there.
    #[intrinsic]
    export fn fetch_and(&self, value: T) -> T;

    /// Puts what's there `|` `value` in it, and gives back what was there.
    #[intrinsic]
    export fn fetch_or(&self, value: T) -> T;

    /// Puts what's there `^` `value` in it, and gives back what was there.
    #[intrinsic]
    export fn fetch_xor(&self, value: T) -> T;

    /// What's in it, which is used up.
    export fn into_inner(self) -> T { self.value }

    /// A reference to what's in it, which is lent as `&mut`, so no other
    /// thread can reach it while the reference lives.
    export fn get_mut(&mut self) -> &mut T { &mut self.value }
}

impl<T: Atom + Int> Atomic<T> {
    /// Adds `value` to what's in it, wrapping around, as `wrapping_add`
    /// does, and gives back what was there.
    #[intrinsic]
    export fn fetch_add(&self, value: T) -> T;

    /// Subtracts `value` from what's in it, wrapping around, and gives
    /// back what was there.
    #[intrinsic]
    export fn fetch_sub(&self, value: T) -> T;
}
