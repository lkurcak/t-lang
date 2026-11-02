//! Shared mutable values: a `Cell` can be changed through a `&`.

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
/// Dropping it drops what's in it, so it's `Discard` if `T` is. It's never
/// `Copy`: a copy would be a second cell, which a change to one doesn't
/// change.
#[derive(Discard where T: Discard)]
export struct Cell<T> { value: T }

impl<T> Cell<T> {
    /// A cell holding `value`.
    export fn new(value: T) -> Cell<T> { Cell { value } }

    /// A copy of what's in the cell.
    export fn get(&self) -> T where T: Copy { self.value }

    /// Puts `value` in the cell, and drops what was there, after: so a
    /// `discard` that drop runs sees `value` in it.
    export fn set(&self, value: T) where T: Discard {
        let old = self.replace(value);
        old.discard();
    }

    /// Puts `value` in the cell, and gives back what was there: how to
    /// change a cell whose type isn't `Discard`.
    #[intrinsic]
    export fn replace(&self, value: T) -> T;

    /// What's in the cell, which is used up.
    export fn into_inner(self) -> T { self.value }

    /// A reference to what's in the cell, which is lent as `&mut`, so
    /// nothing else can reach it while the reference lives.
    export fn get_mut(&mut self) -> &mut T { &mut self.value }
}
