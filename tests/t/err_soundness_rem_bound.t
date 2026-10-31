// `n - 1` is the most a remainder by `n` can be, and it can be that.
fn slot(h: u64, n: { n: u64 | n > 1 }) -> { i: u64 | i < n - 1 } {
    h % n
}

fn main() {
    let _ = slot(4, 5);
}
// error: can't prove `i < n - 1` for the result of `slot`
