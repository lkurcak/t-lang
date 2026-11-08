// A sub-slice's range can't end before it starts.
fn f(s: &[u8]) -> usize {
    if s.len() < 4 {
        return 0;
    }
    let from = 3usize;
    let to = 1usize;
    s.slice(from..to).len()
}

fn main() {}
// error: can't prove `r.start() <= r.end() && r.end() <= self.len()` for argument `r` of `slice`
