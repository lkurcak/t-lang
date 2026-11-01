#[blame_caller]
struct S {}

fn main() {}
// error: `#[blame_caller]` can only be on a function with a body
