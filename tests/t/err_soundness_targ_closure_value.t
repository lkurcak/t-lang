// Nor a closure, whose parameters' types have no refinements.
use alloc::list::List;

fn main() with alloc {
    let mut q: List<{ t: usize | t < 4 }> = List::new();
    let mut f = |l: &mut List<usize>| l.push(9);
    f(&mut q);
}
// error: can't give `&mut q` to a call through a value
