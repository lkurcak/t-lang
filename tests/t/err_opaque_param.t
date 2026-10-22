// `impl` is only for return types: a parameter takes a type parameter.
fn apply(f: impl Fn(i64) -> i64, x: i64) -> i64 { f(x) }

fn main() { print(apply(|x| x, 1)); }
// error: `impl` types can only be written in a function's return type
