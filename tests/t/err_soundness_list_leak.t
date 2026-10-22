// A list isn't `Discard`: dropping it would leak its buffer.
use alloc::list::List;

fn main() with {alloc, panic} {
    let mut l: List<int> = List::new();
    l.push(1);
}
// error: `l` is never used up, and its type, `List<int>`, isn't `Discard`
