// `split_at_mut` borrows the slice for as long as its halves live, so the
// whole can't be changed while a half can.
use alloc::list::List;

fn main() with {alloc, panic} {
    let mut l: List<i64> = List::new();
    l.push(1);
    l.push(2);
    let s = l.as_mut_slice();
    let halves = s.split_at_mut(1).unwrap();
    *s.get_mut(0).unwrap() = 5;
    *halves.head.get_mut(0).unwrap() = 6;
    l.discard();
}
// error: cannot borrow `*s` as mutable more than once at a time
