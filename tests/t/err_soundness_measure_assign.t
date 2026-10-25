// After `s = t`, what was known of `s.len()` is about the slice it was.
fn f(a: &[i64], b: &[i64]) -> i64 {
    let mut s = a;
    if s.len() > 2 {
        s = b;
        return s[2];
    }
    0
}

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
