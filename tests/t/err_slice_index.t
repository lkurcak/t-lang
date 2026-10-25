// `s[i]` needs `i < s.len()` proved, and nothing says how long `empty()`'s
// slice is: `get` gives the value at an index, or `None` if there's none.
fn main() {
    let s: &[i64] = core::slice::empty();
    let _x = s[0];
}
// error: can't prove `i < self.len()` for argument `i` of `index`
