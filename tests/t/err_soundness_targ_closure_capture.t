// A closure that pushes to a refined list it captures checks what it
// pushes, as the list's own code would.
use alloc::list::List;

fn main() with alloc {
    let mut q: List<{ t: usize | t < 4 }> = List::new();
    let mut f = || q.push(7);
    f();
}
// error: can't prove `t < 4` for argument `v` of `push`
