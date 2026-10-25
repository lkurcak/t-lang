// An array's values are values, and a slice is never one.
fn f(a: &[[i64]; 2]) {}

fn main() {}
// error: a slice `[i64]` can only be behind a reference, as in `&[i64]`
