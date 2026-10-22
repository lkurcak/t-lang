// Raw memory: a buffer of slots for values, each there or not.

/// A buffer on the heap of `cap()` slots for values of type `T`, each
/// *initialised* (holding a value) or not. It's made with every slot
/// uninitialised.
///
/// Nothing tracks which slots hold values: the code using it does. So
/// what reads, writes, borrows or frees a slot is `unsafe`, with an
/// obligation its caller must keep, written in its `# Safety`. A broken
/// one is a bug in the `unsafe` block around the call, which the
/// interpreter reports.
///
/// It isn't `Discard`: it's used up by `free`, so its memory is never
/// leaked, nor freed twice.
#[intrinsic]
pub struct RawBuf<T> {}

impl<T> RawBuf<T> {
    /// A buffer of `cap` uninitialised slots.
    #[intrinsic]
    pub fn new(cap: { c: int | c >= 0 }) -> RawBuf<T> with {alloc, panic};

    /// How many slots it has.
    #[intrinsic]
    pub fn cap(&self) -> { c: int | c >= 0 };

    /// Gives it `new_cap` slots, keeping what's in the first `cap()`. The
    /// slots after them are uninitialised. It panics if `new_cap` is less
    /// than `cap()`.
    #[intrinsic]
    pub fn grow(&mut self, new_cap: int) with {alloc, panic};

    /// Puts `v` in slot `i`.
    ///
    /// # Safety
    ///
    /// `i < cap()`, and slot `i` is uninitialised.
    #[intrinsic]
    pub fn write(&mut self, i: { i: int | i >= 0 }, v: T) with unsafe;

    /// Moves the value out of slot `i`, which is uninitialised after.
    ///
    /// # Safety
    ///
    /// `i < cap()`, and slot `i` is initialised.
    #[intrinsic]
    pub fn read(&mut self, i: { i: int | i >= 0 }) -> T with unsafe;

    /// The value in slot `i`.
    ///
    /// # Safety
    ///
    /// `i < cap()`, and slot `i` is initialised.
    #[intrinsic]
    pub fn get(&self, i: { i: int | i >= 0 }) -> &T with unsafe;

    /// The value in slot `i`, to change.
    ///
    /// # Safety
    ///
    /// `i < cap()`, and slot `i` is initialised.
    #[intrinsic]
    pub fn get_mut(&mut self, i: { i: int | i >= 0 }) -> &mut T with unsafe;

    /// Gives its memory back.
    ///
    /// # Safety
    ///
    /// Every slot is uninitialised: what was in them has been read out.
    #[intrinsic]
    pub fn free(self) with unsafe;
}
