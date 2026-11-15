// A call through a function's value, known to be `push9`, is a call of
// `push9`: what it may write behind a `&mut` it's lent must keep what's
// declared there (docs/78).
use alloc::list::List;

fn push9(l: &mut List<usize>) with alloc { l.push(9) }

fn main() with alloc {
    let mut q: List<{ t: usize | t < 4 }> = List::new();
    let f: fn(&mut List<usize>) with alloc = push9;
    f(&mut q);
}
// error: can't prove `t < 4` of what `push9` may write behind argument `l`
