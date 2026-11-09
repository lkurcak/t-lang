// A list of numbers below 4 can't be given a 4: `push`'s `v` is a `T`,
// which here is `{ t: usize | t < 4 }`.
use alloc::list::List;

fn main() with alloc {
    let mut q: List<{ t: usize | t < 4 }> = List::new();
    q.push(4);
}
// error: can't prove `t < 4` for argument `v` of `push`
