// There's no indexing syntax yet: `get` gives the value at an index, or
// `None` if there's none.
fn main() {
    let s: &[int] = core::slice::empty();
    let x = s[0];
}
// error: there's no indexing with `[..]` yet; use `.get(..)`, which gives an `Option`
