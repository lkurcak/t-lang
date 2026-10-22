// Nor take one apart, to free its buffer with values still in it.
use alloc::list::List;

fn main() with {alloc, panic} {
    let mut l: List<int> = List::new();
    l.push(1);
    let List { buf, len } = l;
    // SAFETY: wrong.
    unsafe { buf.free() };
}
// error: field `buf` of `List` is private to `alloc::list`
