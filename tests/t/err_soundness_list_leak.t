// A list isn't `Discard`: dropping it would leak its buffer.
use alloc::list::List;

fn main() with {alloc, panic} {
    let mut l: List<i64> = List::new();
    l.push(1);
}
// error: `l` is never used up, and its type, `List<i64>`, isn't `Discard`
