// Below an enum: a `Some` of an unrefined list isn't a `Some` of a refined
// one.
use alloc::list::List;

fn main() with alloc {
    let mut l: List<usize> = List::new();
    l.push(9);
    let o: Option<List<{ t: usize | t < 4 }>> = Some(l);
    let _ = o;
}
// error: can't prove `t < 4` for `o`
