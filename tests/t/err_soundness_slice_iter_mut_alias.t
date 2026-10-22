// What `iter_mut` gives borrows the slice: it can't be read while one of
// them lives.
use alloc::list::List;

fn main() with {alloc, panic} {
    let mut l: List<int> = List::new();
    l.push(1);
    let s = l.as_mut_slice();
    let mut it = s.iter_mut();
    let a = it.next().unwrap();
    print(*s.get(0).unwrap());
    *a = 2;
    l.free();
}
// error: while `*s` is mutably borrowed
