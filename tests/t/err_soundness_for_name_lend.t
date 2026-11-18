// A function the iterator is lent to can give it anything to go through:
// what's left after is what its signature says, here nothing.
use core::slice::Iter;

fn refill<'a>(it: &mut Iter<'a, u64>, more: &'a [u64]) {
    *it = more.iter();
}

fn count(a: &[u64], b: &[u64]) -> { n: usize | n <= a.len() } with {} {
    let mut n: usize = 0;
    for _x in it: a invariant n + it.len() <= a.len() {
        n = n + 1;
        refill(&mut it, b);
    }
    n
}

fn main() {
    let _ = count(&[0, 1], &[1, 2, 3, 4, 5]);
}
// error: can't prove `n + it.len() <= a.len()` for the loop's invariant
