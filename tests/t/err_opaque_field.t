struct S { f: impl Fn() -> int }

fn main() {}
// error: `impl` types can only be written in a function's return type
