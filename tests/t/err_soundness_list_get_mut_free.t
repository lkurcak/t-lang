// A reference from `get_mut` can't outlive the list's memory.
use alloc::list::List;

fn main() with {alloc, panic} {
    let mut l: List<i64> = List::new();
    l.push(1);
    let r = l.get_mut(0).unwrap();
    l.free();
    *r = 2;
}
// error: cannot move out of `l` while `l` is mutably borrowed
