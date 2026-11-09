// Nor as a `&mut` to a list of numbers below 5, though each below 4 is
// below 5: `f` could push a 4 through it. A `&mut` keeps its refinements
// exactly.
use alloc::list::List;

fn f(l: &mut List<{ t: usize | t < 5 }>) with alloc { l.push(4) }

fn main() with alloc {
    let mut q: List<{ t: usize | t < 4 }> = List::new();
    f(&mut q);
}
// error: can't prove `t < 4` of what `f` may write behind argument `l`
