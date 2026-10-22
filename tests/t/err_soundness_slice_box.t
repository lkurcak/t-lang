// Not even in a box, for now.
fn f(b: Box<[i64]>) {}

fn main() {}
// error: a slice `[i64]` can only be behind a reference, as in `&[i64]`
