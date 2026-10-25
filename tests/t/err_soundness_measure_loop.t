// A loop that changes `s` gives it a new value each time round: what was
// known before the loop isn't known at its head.
fn f(a: &[i64]) -> i64 {
    let mut s = a;
    let mut total: i64 = 0;
    if s.len() > 0 {
        while total < 10 {
            total = core::num::wrapping_add(total, s[0]);
            s = core::slice::empty();
        }
    }
    total
}

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
