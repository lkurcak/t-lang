//! Boxes: values on the heap.

/// A value on the heap, which the box owns: `Box::new(x)` moves `x` there,
/// and `*b` is what box `b` holds, which can be read, changed, or moved
/// out, using the box up. It's how a type can hold itself, as a list's
/// node holds the rest of the list.
///
/// `Box::new` allocates, so it says `with alloc`, and aborts if there's no
/// memory left (see docs/16-out-of-memory.md). A box is `Discard` if what
/// it holds is, and `Eq` if that is, comparing what they hold. It's never
/// `Copy`: a copy would be a second owner.
#[intrinsic]
pub struct Box<T> {}
