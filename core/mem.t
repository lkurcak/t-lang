//! Moving values around without making or throwing away any, and the two
//! traits that say when a value may be copied or thrown away.

/// Says a type's values may be thrown away, rather than used up: when a
/// variable of it goes out of scope, or is overwritten, or `_` matches it.
/// Without it, a value must be passed on, returned, or taken apart, so a
/// value that stands for something to do, like a file to close, can't be
/// forgotten (see docs/06a.1-linear-types.md).
///
/// The integers, `bool`, `char`, `str`, references, functions and slices
/// of `Discard` values have it, and boxes of them. A struct or enum has it
/// if it says so: with `#[derive(Discard)]`, dropping it drops each field;
/// with an impl, it says how itself, in its `discard`, as a list frees its
/// buffer (see docs/20-destructors.md).
#[intrinsic]
export trait Discard {
    /// Throws `self` away. Wherever a value is dropped, its type's
    /// `discard` is what's called: it's total, so it can be called on
    /// every path, the early ones of `?` too. Called by name, it drops
    /// `self` now, rather than at the end of its scope.
    fn discard(self);
}

/// Says a type's values are copied, rather than moved: using one leaves
/// it where it was. It needs `Discard`, as a copy that can't be thrown
/// away would be one more value to use up.
///
/// The integers, `bool`, `char`, shared references and functions have it.
/// A struct or enum has it if it says so, with `#[derive(Copy)]`, and only
/// then.
#[intrinsic]
export trait Copy: Discard {}

/// Exchanges the values `a` and `b` point to. Nothing is made, and nothing
/// thrown away, so it works for any type, `Discard` or not.
#[intrinsic]
export fn swap<T>(a: &mut T, b: &mut T);

/// Puts `src` where `dest` points and gives back what was there: how to
/// change what's behind a `&mut` whose type isn't `Discard`.
export fn replace<T>(dest: &mut T, mut src: T) -> T {
    swap(dest, &mut src);
    src
}
