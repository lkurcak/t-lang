// `split_at`'s head is as long as the index it's given.
fn f(s: &[u8]) -> usize {
    match s.split_at(2) {
        Some(sp) => {
            let n: { n: usize | n == 3 } = sp.0.len();
            n
        },
        None => 0,
    }
}

fn main() {}
// error: can't prove `n == 3` for `n`
