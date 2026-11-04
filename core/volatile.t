//! Memory the program doesn't own: a device's registers, mapped at
//! addresses, which change on their own and do something when they're
//! read or written (see docs/40-extern.md).

use crate::sync::Atom;

/// A value at a fixed address that isn't the program's: a device's
/// register, say. Each `read` and `write` is one access of the machine's,
/// done when and as often as the program says, never left out, merged or
/// moved past another: they're *volatile*.
///
/// A `&Volatile<T>` comes only from `at`, which is `unsafe`: that the
/// address is one is its caller's to know. After that, reading and writing
/// are safe, as they are of a `Cell`: what's there is only copied in and
/// out, nothing points into it, and the checker's logic never looks
/// inside, as it may change between any two reads.
///
/// It's invariant in `T`, never `Copy`, and neither `Send` nor `Sync`: a
/// device shared by two threads, or by a program and its interrupts, needs
/// more than volatile accesses to be used safely.
export struct Volatile<T: Atom> { value: T }

impl<T: Atom> Volatile<T> {
    /// The `T` at `address`, for the whole run.
    ///
    /// # Safety
    ///
    /// `address` is aligned for a `T`, and there's one there, for the whole
    /// run, which nothing but volatile accesses of a `T` read or write: a
    /// device's register, or memory set aside for one. No value of the
    /// program's is there.
    #[intrinsic]
    export fn at(address: usize) -> &'static Volatile<T> with unsafe;

    /// Reads what's there.
    #[intrinsic]
    export fn read(&self) -> T;

    /// Writes `value` there.
    #[intrinsic]
    export fn write(&self, value: T);
}
