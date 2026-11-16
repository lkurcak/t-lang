// A closure that changes what it captured gives, each run, what its body
// knows of a value it doesn't know: the first run's `n` isn't the last's.
fn main() {
    let a: [u8; 8] = [0; 8];
    let mut n: usize = 0;
    for r in (0..4).map(move |x| {
        n = n + 3;
        x + n
    }) {
        let _ = a[r];
    }
}
// error: can't prove
