//! Shared mutable values: what can be changed through a `&`.
//!
//! `Unsafe` is the one primitive: the only type whose contents may change
//! while a `&` to it lives. Everything else that does, `Cell`, the atomics
//! and mutexes of `core::sync`, `core::volatile::Volatile`, alloc's arenas,
//! or a library's own, is a struct with an `Unsafe` in it, and gets what
//! that takes from its fields: it's invariant, not frozen, and not `Sync`,
//! unless it says so with an `unsafe impl` (see docs/46-unsafe-cell.md).

/// A value that may change while a `&` to it lives: the one thing the
/// checker lets change so. Reaching it through a `&` is unsafe, as two
/// `&Unsafe<T>`s could lend it twice: `as_mut` and `as_ref` say what their
/// caller must make sure of. Through a `&mut`, `get_mut` is safe.
///
/// The checker's logic never looks inside: its one field is private to
/// `core::cell`, which has no measure of it (lemma U1 of
/// docs/46-unsafe-cell.md). It's *invariant* in `T`: an
/// `&Unsafe<&'static str>` can't be used as an `&Unsafe<&'a str>`, through
/// which `as_mut` could put in a reference that doesn't live as long as its
/// type says (U2). It isn't *frozen*, so no const holds one (U3).
///
/// It's `Send` if `T` is, but never `Sync`, and only core could say it is:
/// two threads could change it at once, through two `&`s (U4). A type with
/// one in it says it's `Sync` with an `unsafe impl`, as `Mutex` does. It's
/// never `Copy`: a copy would be a second value, which a change to the
/// first doesn't change.
#[derive(Discard where T: Discard, Send where T: Send)]
export struct Unsafe<T> { value: T }

impl<T> Unsafe<T> {
    /// An `Unsafe` holding `value`.
    export fn new(value: T) -> Unsafe<T> { Unsafe { value } }

    /// What's in it, which is used up.
    export fn into_inner(self) -> T { self.value }

    /// A reference to what's in it, which is lent as `&mut`, so nothing
    /// else can reach it while the reference lives.
    export fn get_mut(&mut self) -> &mut T { &mut self.value }

    /// A reference to what's in it, lent as `&mut` though it's only
    /// borrowed by `&`.
    ///
    /// # Safety
    ///
    /// While the reference lives, nothing else reaches what's in it: no
    /// other `as_mut` or `as_ref` reference to it lives, nor is one made.
    #[intrinsic]
    export fn as_mut(&self) -> &mut T with unsafe;

    /// A reference to what's in it, lent as `&`.
    ///
    /// # Safety
    ///
    /// While the reference lives, nothing changes what's in it: no `as_mut`
    /// reference to it lives, nor is one made.
    #[intrinsic]
    export fn as_ref(&self) -> &T with unsafe;
}

/// A value that can be changed through a shared reference, `&Cell<T>`, as
/// well as through a `&mut`. Everything else in T that's behind a `&` stays
/// as it is while the reference lives; what's in a cell doesn't.
///
/// That's sound as nothing ever points *into* a cell: what's in it is only
/// ever copied out (`get`) or moved out (`replace`), so no reference sees it
/// change, and the checker's logic never looks inside one, as no measure or
/// refinement can read it (lemma K1 of docs/37-cell.md).
///
/// It's *invariant* in `T`: a `&Cell<&'static str>` can't be used as a
/// `&Cell<&'a str>`, through which `set` would put in a reference that
/// doesn't live as long as the cell's type says (lemma K2).
///
/// It's `Send` if `T` is, but never `Sync`: two threads could change it
/// at once, through two `&`s (lemma Y1 of docs/38-statics.md).
///
/// Dropping it drops what's in it, so it's `Discard` if `T` is. It's never
/// `Copy`: a copy would be a second cell, which a change to one doesn't
/// change.
///
/// It's an `Unsafe`, lent only for as long as a copy or swap takes, which
/// is all it takes from it: invariance, and not being frozen or `Sync`.
#[derive(Discard where T: Discard, Send where T: Send)]
export struct Cell<T> { value: Unsafe<T> }

impl<T> Cell<T> {
    /// A cell holding `value`.
    export fn new(value: T) -> Cell<T> { Cell { value: Unsafe::new(value) } }

    /// A copy of what's in the cell.
    export fn get(&self) -> T where T: Copy {
        // SAFETY: the `&` lives only for the copy, which runs no code of the
        // program's, so nothing lends what's in the cell meanwhile.
        unsafe { *self.value.as_ref() }
    }

    /// Puts `value` in the cell, and drops what was there, after: so a
    /// `discard` that drop runs sees `value` in it.
    export fn set(&self, value: T) where T: Discard {
        let old = self.replace(value);
        old.discard();
    }

    /// Puts `value` in the cell, and gives back what was there: how to
    /// change a cell whose type isn't `Discard`.
    export fn replace(&self, value: T) -> T {
        // SAFETY: the `&mut` lives only for the swap, which runs no code of
        // the program's (no `discard`: the old value is moved out), so
        // nothing lends what's in the cell meanwhile.
        crate::mem::replace(unsafe { self.value.as_mut() }, value)
    }

    /// What's in the cell, which is used up.
    export fn into_inner(self) -> T { self.value.into_inner() }

    /// A reference to what's in the cell, which is lent as `&mut`, so
    /// nothing else can reach it while the reference lives.
    export fn get_mut(&mut self) -> &mut T { self.value.get_mut() }
}
