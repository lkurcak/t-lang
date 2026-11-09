// An empty list has every refinement of its elements, but only until
// something's pushed: a `let` without one declares nothing of them.
use alloc::list::List;

fn main() with alloc {
    let mut l = List::new();
    l.push(9usize);
    let k: List<{ v: usize | v < 3 }> = l;
    let _ = k;
}
// error: can't prove `v < 3`
