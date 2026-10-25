// A measure is of a value: two slices with equal lengths are equal in
// length, but nothing says two different ones are.
fn f(a: &[i64], b: &[i64]) -> i64 {
    if a.len() > 0 {
        return b[0];
    }
    0
}

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
