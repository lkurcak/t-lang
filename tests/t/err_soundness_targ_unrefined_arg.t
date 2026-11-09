// An unrefined list isn't a list of numbers below 4.
use alloc::list::List;

fn f(l: List<{ t: usize | t < 4 }>) { let _ = l; }

fn main() with alloc {
    let mut l: List<usize> = List::new();
    l.push(9);
    f(l);
}
// error: can't prove `t < 4` for argument `l` of `f`
