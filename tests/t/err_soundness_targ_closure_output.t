// A closure that gives back a `T` can't count on its parameter having the
// other arguments' refinement: `twice` gives it what it gave.
fn twice<T, F: FnMut(T) -> T + Discard>(x: T, mut f: F) -> T { f(f(x)) }

fn main() {
    let a: [u8; 4] = [1, 2, 3, 4];
    let i: { v: usize | v < 4 } = 1;
    let _ = twice(i, |t| {
        let _ = a[t];
        t + 5
    });
}
// error: can't prove `i < self.len()` for argument `i` of `index`
