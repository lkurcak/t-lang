// The body gives the iterator a new value: what's left is then the new
// one's, and the invariant must hold of that, as of any variable.
fn count(a: &[u64], b: &[u64]) -> { n: usize | n <= a.len() } with {} {
    let mut n: usize = 0;
    for x in it: a invariant n + it.len() <= a.len() {
        n = n + 1;
        if *x == 0 {
            it = b.iter();
        }
    }
    n
}

fn main() {
    let _ = count(&[0, 1], &[1, 2, 3, 4, 5]);
}
// error: can't prove `n + it.len() <= a.len()` for the loop's invariant
