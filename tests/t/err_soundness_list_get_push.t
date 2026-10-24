// `push` may grow the buffer, moving its slots, so a reference from `get`
// can't live across it: `get` borrows the list, `push` needs it `&mut`.
use alloc::list::List;

fn main() with {alloc, panic} {
    let mut l: List<i64> = List::new();
    l.push(1);
    let r = l.get(0);
    l.push(2);
    let _v = *r.unwrap();
    l.free();
}
// error: cannot borrow `l` as mutable while `l` is borrowed
