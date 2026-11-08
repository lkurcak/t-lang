// Where branches meet, a slice is whichever branch's it is: its length is
// one of theirs, not the first's.
fn f(s: &[u8], c: bool) -> usize {
    if s.len() < 4 {
        return 0;
    }
    let t = if c { s.slice(0..2) } else { s.slice(0..3) };
    let n: { n: usize | n == 2 } = t.len();
    n
}

fn main() {}
// error: can't prove `n == 2` for `n`
