// Nor as a type argument.
fn main() {
    let x: Option<[i64]> = Option::None;
}
// error: a slice `[i64]` can only be behind a reference, as in `&[i64]`
