// A slice is never a value of its own, only behind a reference: how many
// values `*s` is isn't known until the program runs.
fn main() {
    let s: &[int] = core::slice::empty();
    let v = *s;
}
// error: a slice `[int]` can only be behind a reference, as in `&[int]`
