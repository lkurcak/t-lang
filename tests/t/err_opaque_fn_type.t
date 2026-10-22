// Not in a function type either, even in a return type: `fn() -> impl ..`
// would be a different type for each function it holds.
fn f() -> fn() -> impl Fn() -> i64 { f }

fn main() {}
// error: `impl` types can only be written in a function's return type
