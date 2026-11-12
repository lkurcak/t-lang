// A closure that pushes to the list it captures changes its length: its
// loan keeps nothing of it.
use alloc::list::List;

fn f(l: &mut List<u8>) with alloc ensures l.len() == old(l.len()) {
    let mut p = |v: u8| l.push(v);
    p(1);
}

fn main() {}
// error: can't prove `l.len() == old(l.len())` for what `f` ensures
