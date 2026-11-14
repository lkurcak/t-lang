// A closure that changes what it captured by value runs with what the
// runs before left there, not with the value it was made with: each run
// reads a value nothing's known of but its refinements (since 0.84.0,
// each run read `n` as 0, and the third indexed `a[4]`).
fn thrice<F: FnMut() -> u32 + Discard>(mut f: F) -> u32 {
    let _ = f();
    let _ = f();
    f()
}

fn main() {
    let a: [u32; 3] = [1, 2, 3];
    let mut n: usize = 0;
    let _ = thrice(move || {
        let r = a[n];
        n = n + 2;
        r
    });
}
// error: can't prove `i < self.len()` for argument `i` of `index`
