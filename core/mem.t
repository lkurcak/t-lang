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
/// if it says so, with `#[derive(Discard)]`, and only then: it can't be
/// implemented by hand.
#[intrinsic]
pub trait Discard {}

/// Says a type's values are copied, rather than moved: using one leaves
/// it where it was. It needs `Discard`, as a copy that can't be thrown
/// away would be one more value to use up.
///
/// The integers, `bool`, `char`, shared references and functions have it.
/// A struct or enum has it if it says so, with `#[derive(Copy)]`, and only
/// then.
#[intrinsic]
pub trait Copy: Discard {}

/// Exchanges the values `a` and `b` point to. Nothing is made, and nothing
/// thrown away, so it works for any type, `Discard` or not.
#[intrinsic]
pub fn swap<T>(a: &mut T, b: &mut T);

/// Puts `src` where `dest` points and gives back what was there: how to
/// change what's behind a `&mut` whose type isn't `Discard`.
pub fn replace<T>(dest: &mut T, mut src: T) -> T {
    swap(dest, &mut src);
    src
}
