// `[T]` is a type only right after `&` or `&mut`.
fn f(s: [int]) {}

fn main() {}
// error: a slice `[int]` can only be behind a reference, as in `&[int]`
