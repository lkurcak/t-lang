// A slice can't outlive the list it's of.
use alloc::list::List;

fn main() with alloc {
    let mut l: List<i64> = List::new();
    l.push(1);
    let s = l.as_slice();
    l.free();
    print(s.len());
}
// error: cannot move out of `l` while `l` is borrowed
