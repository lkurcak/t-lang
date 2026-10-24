use alloc::boxed::Box;

#[derive(Discard, Copy)]
struct B { b: Box<i64> }
fn main() {}
// error: `B` can't be `Copy`: its field `b` holds a `Box<i64>`, which isn't
