// A loop's iterator forgets what's only known of the values now, but
// keeps that an empty list has none: joined with a list with a 9 in it,
// what the loop gives may be the 9.
use alloc::list::List;

fn main() with alloc {
    let mut l = List::new();
    l.push(9usize);
    let b = l.len() == 1;
    let w = if b { l } else { List::new() };
    for v in w.iter() {
        let _k: { k: usize | k < 3 } = *v;
    }
}
// error: can't prove `k < 3` for `_k`
