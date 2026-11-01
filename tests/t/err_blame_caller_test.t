// A test is run as a program of its own: it has no caller to blame.
fn main() {}

#[blame_caller]
#[test]
fn t() { assert(false); }
// error: a test can't blame its caller
