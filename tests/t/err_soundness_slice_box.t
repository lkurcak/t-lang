// Not even in a box, for now.
fn f(b: Box<[int]>) {}

fn main() {}
// error: a slice `[int]` can only be behind a reference, as in `&[int]`
