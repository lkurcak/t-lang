// A const parameter is a length, so a `usize`.
fn f<const N: i64>() {}

fn main() {}
// error: a const parameter must be a `usize`
