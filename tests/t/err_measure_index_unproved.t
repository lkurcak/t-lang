// `s[i]` needs `i < s.len()`, and nothing says so.
fn at(s: &[i64], i: usize) -> i64 { s[i] }

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
