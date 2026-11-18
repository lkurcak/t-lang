// Taking a value out in the body skips it: an index kept beside the loop
// is then more than how far it's got, so `i + it.len() == a.len()` doesn't
// hold at the end of the body, and `b[i]` could be past the end.
fn same(a: &[u64], b: { b: &[u64] | b.len() == a.len() }) -> usize with {} {
    let mut i: usize = 0;
    let mut n: usize = 0;
    for x in it: a invariant i + it.len() == a.len() && n <= i {
        if *x == b[i] {
            n = n + 1;
        }
        let _ = it.next();
        i = i + 1;
    }
    n
}

fn main() {
    let _ = same(&[1, 2, 3], &[1, 0, 3]);
}
// error: can't prove `i + it.len() == a.len() && n <= i` for the loop's invariant
