// `None` has no values in its `Some` field, but joined with a list that
// has a 9 in it, the result can: so it isn't a list of numbers below 3.
use alloc::list::List;

fn main() with alloc {
    let mut l = List::new();
    l.push(9usize);
    let b = l.len() == 1;
    let o = if b { Some(l) } else { None };
    let k: Option<List<{ v: usize | v < 3 }>> = o;
    let _ = k;
}
// error: can't prove `v < 3`
