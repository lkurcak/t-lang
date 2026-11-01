// `main` has no caller in the program to blame.
#[blame_caller]
fn main() with panic { panic("no") }
// error: `main` can't blame its caller
