//! Arenas: values that live as long as the arena they're put in, and go
//! together, when it does.

/// Values of type `T` on the heap, all freed at once, with the arena.
/// `alloc` moves a value in and gives a reference to it there, which lives
/// as long as the borrow of the arena, so values in it can point to each
/// other: a list's nodes, or a tree's, without a box for each.
///
/// It hands out `&mut`s from a `&`, so many can be had at once. That's
/// sound as each is to a value no other reference has been to, and nothing
/// else reaches the values in it until it's dropped (see
/// docs/23-arenas.md). For the same reason it's *invariant* in `T`: an
/// `&Arena<&'static str>` can't be used as an `&Arena<&'a str>`, to put
/// in what doesn't live as long as the arena's type says.
///
/// Dropping it drops each value in it, in the order they were put in, so
/// it's `Discard` if `T` is. It's never `Copy`.
#[derive(Discard where T: Discard)]
#[intrinsic]
pub struct Arena<T> {}

impl<T> Arena<T> {
    /// An empty arena. It allocates nothing until something's put in.
    #[intrinsic]
    pub fn new() -> Arena<T>;

    /// Moves `v` into the arena, and gives a reference to it there. Like
    /// everything that allocates, it aborts if there isn't the memory.
    #[intrinsic]
    pub fn alloc(&self, v: T) -> &mut T with alloc;

    /// How many values have been put in.
    #[intrinsic]
    pub fn len(&self) -> usize;
}
