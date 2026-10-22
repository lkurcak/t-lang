// `[T]` is a type only right after `&` or `&mut`.
fn f(s: [i64]) {}

fn main() {}
// error: a slice `[i64]` can only be behind a reference, as in `&[i64]`
