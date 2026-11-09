// `swap` exchanges two values of one type: a refined list's with an
// unrefined one's would put any numbers in it.
use alloc::list::List;
use core::mem::swap;

fn main() with alloc {
    let mut q: List<{ t: usize | t < 4 }> = List::new();
    let mut other: List<usize> = List::new();
    other.push(9);
    swap(&mut q, &mut other);
}
// error: can't prove `t < 4` of what `swap` may write behind argument `a`
