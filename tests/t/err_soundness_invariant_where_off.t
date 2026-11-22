// And isn't assumed where it isn't: of an iterator that may never end,
// counting may overflow (docs/92-where-a-bound-holds.md).
use core::num::greatest;

fn count<I: Iterator + Discard>(it: I) -> usize with I::e where I::Item: Discard {
    let mut n: usize = 0;
    for _x in jt: it invariant n + jt.most() <= greatest::<usize>() where I: BoundedIterator {
        n = n + 1;
    }
    n
}

fn main() {}
// error: `+` may panic, but `count` doesn't say `with panic`
