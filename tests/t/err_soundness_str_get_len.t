// What `str::get` gives is as long as from `lo` to `hi`.
fn f(s: &str) -> usize {
    match s.get(1, 3) {
        Some(t) => {
            let n: { n: usize | n == 3 } = t.len();
            n
        },
        None => 0,
    }
}

fn main() {}
// error: can't prove `n == 3` for `n`
