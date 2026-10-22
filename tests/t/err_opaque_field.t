struct S { f: impl Fn() -> i64 }

fn main() {}
// error: `impl` types can only be written in a function's return type
