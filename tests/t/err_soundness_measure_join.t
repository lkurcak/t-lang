// After a branch that may have changed `s`, it's whichever slice the branch
// taken gave it.
fn f(a: &[i64], b: &[i64], c: bool) -> i64 {
    let mut s = a;
    if a.len() > 1 {
        if c {
            s = b;
        }
        return s[1];
    }
    0
}

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
