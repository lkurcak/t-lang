//! Raw memory: a buffer of slots for values, each there or not.

/// Aborts the program, as allocating does when there's no memory left.
/// It's for what can't go on for lack of memory without allocating: a
/// list whose length can't count one more value, say. Running out of
/// memory is part of `alloc`, not `panic`, which is for bugs (see
/// docs/16-out-of-memory.md).
#[intrinsic]
export fn out_of_memory() -> ! with alloc;

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
/// Its methods say what they do to `cap()`: only `grow` changes it. That's
/// what lets a `List` keep `len <= cap()` as a refinement of its fields
/// while it writes its values (see docs/30-postconditions.md).
///
/// It isn't `Discard`: it's used up by `free`, so its memory is never
/// leaked, nor freed twice.
#[intrinsic]
export struct RawBuf<T> {}

impl<T> RawBuf<T> {
    /// A buffer of `cap` uninitialised slots. Like everything that
    /// allocates, it aborts if there isn't the memory for them.
    #[intrinsic]
    export fn new(cap: usize) -> { b: RawBuf<T> | b.cap() == cap } with alloc;

    /// A buffer of no slots. It allocates nothing, so it can't fail, and
    /// `grow` gives it some.
    #[intrinsic]
    export fn empty() -> { b: RawBuf<T> | b.cap() == 0 };

    /// How many slots it has. It's a measure, so refinements can say it.
    #[intrinsic]
    #[measure]
    export fn cap(&self) -> usize;

    /// Gives it `extra` more slots, keeping what's in the first `cap()`.
    /// The slots after them are uninitialised. More slots than a `usize`
    /// can count are more memory than there is, so it aborts then, as it
    /// does when there isn't the memory.
    #[intrinsic]
    export fn grow(&mut self, extra: usize)
    with alloc
    ensures
        self.cap() == old(self.cap()) + extra;

    /// Puts `v` in slot `i`.
    ///
    /// # Safety
    ///
    /// Slot `i` is uninitialised.
    #[intrinsic]
    export fn write(&mut self, i: { i: usize | i < self.cap() }, v: T)
    with unsafe
    ensures
        self.cap() == old(self.cap());

    /// Moves the value out of slot `i`, which is uninitialised after.
    ///
    /// # Safety
    ///
    /// Slot `i` is initialised.
    #[intrinsic]
    export fn read(&mut self, i: { i: usize | i < self.cap() }) -> T
    with unsafe
    ensures
        self.cap() == old(self.cap());

    /// The values in slots `lo` up to, not including, `hi`, as a slice.
    ///
    /// # Safety
    ///
    /// Slots `lo..hi` are initialised, and stay so, unchanged, while the
    /// slice lives.
    #[intrinsic]
    export fn slice(
        &self,
        lo: usize,
        hi: { h: usize | h >= lo && h <= self.cap() },
    ) -> { s: &[T] | s.len() == hi - lo } with unsafe;

    /// The values in slots `lo` up to, not including, `hi`, as a slice to
    /// change.
    ///
    /// # Safety
    ///
    /// Slots `lo..hi` are initialised. The slice borrows the buffer, so
    /// they stay so while it lives: nothing done through it can take a
    /// value out. Nor can it change `cap()`, which `ensures` says of the
    /// buffer as it is when the borrow ends, as only an intrinsic can (see
    /// docs/30-postconditions.md).
    #[intrinsic]
    export fn slice_mut(
        &mut self,
        lo: usize,
        hi: { h: usize | h >= lo && h <= self.cap() },
    ) -> { s: &mut [T] | s.len() == hi - lo } with unsafe ensures self.cap() == old(self.cap());

    /// Gives its memory back.
    ///
    /// # Safety
    ///
    /// Every slot is uninitialised: what was in them has been read out.
    #[intrinsic]
    export fn free(self) with unsafe;
}
