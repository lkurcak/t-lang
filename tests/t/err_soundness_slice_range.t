// A sub-slice's range must be in the slice.
fn f(s: &[u8]) -> u8 {
    if s.len() < 4 {
        return 0;
    }
    s.slice(1..5)[0]
}

fn main() {}
// error: can't prove `r.start() <= r.end() && r.end() <= self.len()` for argument `r` of `slice`
