// Pushing while iterating could move the slots the iterator is reading.
use alloc::list::List;

fn main() with {alloc, panic} {
    let mut l: List<int> = List::new();
    l.push(1);
    for x in l.iter() {
        l.push(*x);
    }
    l.free();
}
// error: cannot borrow `l` as mutable while `l` is borrowed
