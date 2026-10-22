// Nor as a type argument.
fn main() {
    let x: Option<[int]> = Option::None;
}
// error: a slice `[int]` can only be behind a reference, as in `&[int]`
