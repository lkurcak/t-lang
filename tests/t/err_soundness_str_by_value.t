// A `str` is never a value of its own, only behind a reference: how many
// bytes it is isn't known until the program runs.
fn main() {
    let s = "hello";
    let v = *s;
}
// error: `str` can only be behind a shared reference, as in `&str`
