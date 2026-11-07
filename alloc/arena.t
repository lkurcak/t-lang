//! Arenas: values that live as long as the arena they're put in, and go
//! together, when it does.

use core::cell::Unsafe;

/// Values of type `T` on the heap, all freed at once, with the arena.
/// `alloc` moves a value in and gives a reference to it there, which lives
/// as long as the borrow of the arena, so values in it can point to each
/// other: a list's nodes, or a tree's, without a box for each.
///
/// It hands out `&mut`s from a `&`, so many can be had at once. That's
/// sound as each is to a value no other reference has been to, and nothing
/// else reaches the values in it until it's dropped (see
/// docs/23-arenas.md). It keeps them in an `Unsafe` (lemma U6 of
/// docs/46-unsafe-cell.md), so it's *invariant* in `T`: an
/// `&Arena<&'static str>` can't be used as an `&Arena<&'a str>`, to put in
/// what doesn't live as long as the arena's type says.
///
/// Dropping it drops each value in it, in the order they were put in, so
/// it's `Discard` if `T` is. It's never `Copy`. It's `Send` if `T` is,
/// but never `Sync`, as `alloc` changes it through a `&`, which its
/// `Unsafe` shows (lemma U4).
#[derive(Discard where T: Discard, Send where T: Send)]
export struct Arena<T> { slots: Unsafe<Slots<T>> }

impl<T> Arena<T> {
    /// An empty arena. It allocates nothing until something's put in.
    export fn new() -> Arena<T> { Arena { slots: Unsafe::new(Slots::new()) } }

    /// Moves `v` into the arena, and gives a reference to it there. Like
    /// everything that allocates, it aborts if there isn't the memory.
    export fn alloc(&self, v: T) -> &mut T with alloc {
        // SAFETY: no other reference to the slots lives: `alloc` and `len`
        // lend them only while they run, and putting `v` in runs no code of
        // the program's. What's lent out is a value no other reference has
        // been to, which stays where it is as more are put in.
        let slots = unsafe { self.slots.as_mut() };
        slots.alloc(v)
    }

    /// How many values have been put in.
    export fn len(&self) -> usize {
        // SAFETY: the `&` to the slots lives only for `len`, which no
        // `alloc` runs during.
        let slots = unsafe { self.slots.as_ref() };
        slots.len()
    }
}

/// Where an arena's values are: each in a box of its own, which stays
/// where it is as more are put in, all of them dropped, first put in first,
/// when the slots are. Only the compiler looks inside.
#[derive(Discard where T: Discard, Send where T: Send)]
#[intrinsic]
struct Slots<T> {}

impl<T> Slots<T> {
    /// No values. It allocates nothing until something's put in.
    #[intrinsic]
    fn new() -> Slots<T>;

    /// Moves `v` into a box of its own, and gives a reference to it there.
    #[intrinsic]
    fn alloc(&mut self, v: T) -> &mut T with alloc;

    /// How many values have been put in.
    #[intrinsic]
    fn len(&self) -> usize;
}
