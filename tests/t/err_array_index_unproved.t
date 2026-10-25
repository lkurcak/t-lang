// `a[i]` needs `i < a.len()`, which is `i < 3`, and nothing says so.
fn at(a: [i64; 3], i: usize) -> i64 { a[i] }

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
