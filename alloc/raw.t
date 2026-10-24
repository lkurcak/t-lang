//! Raw memory: a buffer of slots for values, each there or not.

/// Aborts the program, as allocating does when there's no memory left.
/// It's for what can't go on for lack of memory without allocating: a
/// list whose length can't count one more value, say. Running out of
/// memory is part of `alloc`, not `panic`, which is for bugs (see
/// docs/16-out-of-memory.md).
#[intrinsic]
pub fn out_of_memory() -> ! with alloc;

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
    /// A buffer of `cap` uninitialised slots. Like everything that
    /// allocates, it aborts if there isn't the memory for them.
    #[intrinsic]
    pub fn new(cap: usize) -> RawBuf<T> with alloc;

    /// A buffer of no slots. It allocates nothing, so it can't fail, and
    /// `grow` gives it some.
    #[intrinsic]
    pub fn empty() -> RawBuf<T>;

    /// How many slots it has.
    #[intrinsic]
    pub fn cap(&self) -> usize;

    /// Gives it `extra` more slots, keeping what's in the first `cap()`.
    /// The slots after them are uninitialised. More slots than a `usize`
    /// can count are more memory than there is, so it aborts then, as it
    /// does when there isn't the memory.
    #[intrinsic]
    pub fn grow(&mut self, extra: usize) with alloc;

    /// Puts `v` in slot `i`.
    ///
    /// # Safety
    ///
    /// `i < cap()`, and slot `i` is uninitialised.
    #[intrinsic]
    pub fn write(&mut self, i: usize, v: T) with unsafe;

    /// Moves the value out of slot `i`, which is uninitialised after.
    ///
    /// # Safety
    ///
    /// `i < cap()`, and slot `i` is initialised.
    #[intrinsic]
    pub fn read(&mut self, i: usize) -> T with unsafe;

    /// The values in slots `lo` up to, not including, `hi`, as a slice.
    ///
    /// # Safety
    ///
    /// `lo <= hi <= cap()`, and slots `lo..hi` are initialised. The slice
    /// borrows the buffer, so they stay so while it lives.
    #[intrinsic]
    pub fn slice(&self, lo: usize, hi: usize) -> &[T] with unsafe;

    /// The values in slots `lo` up to, not including, `hi`, as a slice to
    /// change.
    ///
    /// # Safety
    ///
    /// `lo <= hi <= cap()`, and slots `lo..hi` are initialised. The slice
    /// borrows the buffer, so they stay so while it lives: nothing it does
    /// can take a value out.
    #[intrinsic]
    pub fn slice_mut(&mut self, lo: usize, hi: usize) -> &mut [T] with unsafe;

    /// Gives its memory back.
    ///
    /// # Safety
    ///
    /// Every slot is uninitialised: what was in them has been read out.
    #[intrinsic]
    pub fn free(self) with unsafe;
}
