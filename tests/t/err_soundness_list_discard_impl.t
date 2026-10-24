// Only `alloc` could say how a list is thrown away: another `discard`
// could skip freeing its buffer, or free it twice.
use alloc::list::List;

impl<T> Discard for List<T> {
    fn discard(self) {}
}

fn main() {}
// error: `List<T>`'s `discard` must be written in `alloc::list`, where it's declared
