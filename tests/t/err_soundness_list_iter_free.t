// An iterator borrows its list, so the list can't be freed under it.
use alloc::list::List;

fn main() with {alloc, panic} {
    let mut l: List<int> = List::new();
    l.push(1);
    let mut it = l.iter();
    l.free();
    print(*it.next().unwrap());
}
// error: cannot move out of `l` while `l` is borrowed
