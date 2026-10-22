// A slice is never a value of its own, only behind a reference: how many
// values `*s` is isn't known until the program runs.
fn main() {
    let s: &[i64] = core::slice::empty();
    let v = *s;
}
// error: a slice `[i64]` can only be behind a reference, as in `&[i64]`
