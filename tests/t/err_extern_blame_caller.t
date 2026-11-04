// C doesn't say where it called from.
#[blame_caller]
extern fn f() with panic { core::panic::panic("no") }
// error: `#[blame_caller]` can't be on an `extern fn`

fn main() {}
