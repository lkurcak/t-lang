// A sub-slice is as long as its range, no longer.
fn f(s: &[u8]) -> usize {
    if s.len() < 4 {
        return 0;
    }
    let n: { n: usize | n == 3 } = s.slice(1..3).len();
    n
}

fn main() {}
// error: can't prove `n == 3` for `n`
