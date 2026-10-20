// Moving values around without making or throwing away any.

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
