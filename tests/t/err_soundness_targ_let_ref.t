// A `&mut` to a refined list, in a variable, has the list's refinements:
// what's pushed through it must have them.
use alloc::list::List;

fn main() with alloc {
    let mut q: List<{ t: usize | t < 4 }> = List::new();
    let r = &mut q;
    r.push(4);
}
// error: can't prove `t < 4` for argument `v` of `push`
