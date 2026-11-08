// An invariant reads what it names at the loop's head, each time round:
// not a variable lent `&mut` across it (decision D4 of
// docs/54-loop-invariants.md), even where what it says is true anyway.
use alloc::list::List;

fn f(n: usize) with alloc {
    let mut l: List<u8> = List::new();
    let r = &mut l;
    let mut i: usize = 0;
    while i < n invariant l.len() >= 0 {
        r.push(0u8);
        i = i + 1;
    }
    r.push(1u8);
}

fn main() {}
// error: cannot use `l` while `l` is mutably borrowed
