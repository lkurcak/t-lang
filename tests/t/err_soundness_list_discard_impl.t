// Only `alloc` could say a list is `Discard`, and it doesn't: one dropped
// would leak its buffer.
use alloc::list::List;

impl<T> Discard for List<T> {}

fn main() {}
// error: `Discard` can only be derived
