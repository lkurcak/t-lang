use alloc::boxed::Box;

fn take(r: &Box<i64>) -> Box<i64> { *r }

fn main() {}
// error: cannot move out of `*r`, which is behind a reference
