#[blame_caller(x)]
fn f() with panic {}

fn main() {}
// error: `#[blame_caller]` takes nothing
