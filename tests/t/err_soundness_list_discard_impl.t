// Only `alloc` could say a list is `Discard`, and it doesn't: one dropped
// would leak its buffer.
use alloc::list::List;

impl<T> Discard for List<T> {}

fn main() {}
// error: `Discard` is `core`'s and `List` is `alloc`'s, so only `core` or `alloc` can implement one for the other; to implement it for a type of your own, wrap it in a struct
