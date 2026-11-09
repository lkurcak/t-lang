// A `&mut` to a list of numbers below 4 can't be given as a `&mut
// List<usize>`: `f` could push any number through it.
use alloc::list::List;

fn f(l: &mut List<usize>) with alloc { l.push(9) }

fn main() with alloc {
    let mut q: List<{ t: usize | t < 4 }> = List::new();
    f(&mut q);
}
// error: can't prove `t < 4` of what `f` may write behind argument `l`
