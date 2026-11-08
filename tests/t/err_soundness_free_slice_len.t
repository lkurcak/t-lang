// A free theorem says what a generic function gives is one of its
// arguments: so its length is one of theirs, either.
fn pick<T: Copy>(c: bool, a: T, b: T) -> T {
    if c { a } else { b }
}

fn f(s: &[u8], c: bool) -> usize {
    if s.len() < 4 {
        return 0;
    }
    let t = pick(c, s.slice(0..2), s.slice(0..3));
    let n: { n: usize | n == 2 } = t.len();
    n
}

fn main() {}
// error: can't prove `n == 2` for `n`
