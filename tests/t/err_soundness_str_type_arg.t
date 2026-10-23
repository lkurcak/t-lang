// Nor as a type argument.
fn main() {
    let x: Option<str> = Option::None;
}
// error: `str` can only be behind a shared reference, as in `&str`
