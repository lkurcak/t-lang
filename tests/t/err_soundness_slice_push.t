// `push` may move the list's values somewhere new, so a slice of them
// can't live across it.
use alloc::list::List;

fn main() with alloc {
    let mut l: List<i64> = List::new();
    l.push(1);
    let s = l.as_slice();
    l.push(2);
    let _v = s.len();
    l.free();
}
// error: cannot borrow `l` as mutable while `l` is borrowed
